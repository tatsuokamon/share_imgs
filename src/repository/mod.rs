use aws_sdk_s3::{
    error::SdkError, operation::put_object::PutObjectError, presigning::PresigningConfigError,
};
use bb8::PooledConnection;
use bb8_redis::RedisConnectionManager;
use chrono::NaiveDateTime;
use redis::{AsyncCommands, RedisError};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect,
    RelationTrait,
};
use sha2::Digest;
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

use crate::entity::{comment, image_vote, images, room, user};

#[derive(Debug, thiserror::Error)]
pub enum RepositoryErr {
    #[error("RepositoryErr: DBErr: {0}")]
    DBErr(#[from] DbErr),

    #[error("RepositoryErr: SDKError: {0}")]
    SDKError(#[from] SdkError<PutObjectError>),

    #[error("RepositoryErr: PresigningConfigErr: {0}")]
    PresigningConfigErr(#[from] PresigningConfigError),

    #[error("RepositoryErr: RedisError: {0}")]
    RedisError(#[from] RedisError),
}

fn generate_redis_post_img_tag(user_id: &Uuid) -> String {
    format!("post-img-{}", user_id)
}

fn generate_redis_post_comment_tag(user_id: &Uuid) -> String {
    format!("post-comment-{}", user_id)
}

pub async fn check_if_he_exists(
    db: &impl ConnectionTrait,
    user_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    Ok(user::Entity::find()
        .filter(user::Column::Id.eq(user_id.to_string()))
        .one(db)
        .await?
        .is_some())
}

pub async fn check_if_comment_exists(
    db: &impl ConnectionTrait,
    comment_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    Ok(comment::Entity::find()
        .filter(comment::Column::Id.eq(comment_id.to_string()))
        .one(db)
        .await?
        .is_some())
}

pub async fn check_if_he_is_authorized(
    db: &impl ConnectionTrait,
    user_id: &Uuid,
    room_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    Ok(
        if let Some(m) = room::Entity::find()
            .filter(room::Column::Id.eq(room_id.to_string()))
            .one(db)
            .await?
        {
            m.master_id == *user_id
        } else {
            false
        },
    )
}

pub async fn check_if_room_exists(
    db: &impl ConnectionTrait,
    room_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    Ok(room::Entity::find()
        .filter(room::Column::Id.eq(room_id.to_string()))
        .one(db)
        .await?
        .is_some())
}

pub async fn check_is_keyword_available(
    db: &impl ConnectionTrait,
    keyword: &str,
) -> Result<bool, RepositoryErr> {
    Ok(room::Entity::find()
        .filter(room::Column::Keyword.eq(keyword))
        .one(db)
        .await?
        .is_some())
}

pub async fn check_if_img_exists(
    db: &impl ConnectionTrait,
    img_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    Ok(images::Entity::find()
        .filter(images::Column::Id.eq(img_id.to_string()))
        .one(db)
        .await?
        .is_some())
}

pub async fn check_if_img_deleted(
    db: &impl ConnectionTrait,
    img_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    Ok(
        if let Some(m) = images::Entity::find()
            .filter(images::Column::Id.eq(img_id.to_string()))
            .one(db)
            .await?
            && m.deleted_at != None
        {
            true
        } else {
            false
        },
    )
}

pub async fn generate_user(db: &impl ConnectionTrait) -> Result<Uuid, RepositoryErr> {
    Ok(user::ActiveModel {
        created_at: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await?
    .id)
}

pub async fn generate_room(
    db: &impl ConnectionTrait,
    keyword: String,
    master_id: Uuid,
) -> Result<(), RepositoryErr> {
    room::ActiveModel {
        keyword: sea_orm::ActiveValue::Set(keyword),
        master_id: sea_orm::ActiveValue::Set(master_id),
        created_at: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
        deleted_at: sea_orm::ActiveValue::Set(None as Option<NaiveDateTime>),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

pub async fn get_room_id_from_keyword(
    db: &impl ConnectionTrait,
    keyword: &str,
) -> Result<Option<Uuid>, RepositoryErr> {
    Ok(
        if let Some(m) = room::Entity::find()
            .filter(room::Column::Keyword.eq(keyword))
            .filter(room::Column::DeletedAt.eq(None as Option<NaiveDateTime>))
            .one(db)
            .await?
        {
            Some(m.id)
        } else {
            None
        },
    )
}

pub async fn get_posted_imgs(
    db: &impl ConnectionTrait,
    room_id: &Uuid,
) -> Result<Vec<images::Model>, RepositoryErr> {
    Ok(images::Entity::find()
        .join(
            sea_orm::JoinType::InnerJoin,
            images::Relation::Room.def().rev(),
        )
        .filter(room::Column::Id.eq(room_id.to_string()))
        .filter(images::Column::DeletedAt.eq(None as Option<NaiveDateTime>))
        .all(db)
        .await?)
}

pub async fn get_posted_comments(
    db: &impl ConnectionTrait,
    room_id: &Uuid,
) -> Result<Vec<comment::Model>, RepositoryErr> {
    Ok(comment::Entity::find()
        .join(
            sea_orm::JoinType::InnerJoin,
            comment::Relation::Room.def().rev(),
        )
        .filter(room::Column::Id.eq(room_id.to_string()))
        .filter(comment::Column::DeletedAt.eq(None as Option<NaiveDateTime>))
        .all(db)
        .await?)
}

pub fn generate_object_key(room_id: &Uuid) -> String {
    format!("{}/{}", room_id, Uuid::new_v4().to_string())
}

pub async fn generate_presigned_url(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    object_key: &str,
    expires_in: u64,
) -> Result<String, RepositoryErr> {
    let presigned = client
        .put_object()
        .bucket(bucket)
        .key(object_key)
        .presigned(aws_sdk_s3::presigning::PresigningConfig::expires_in(
            Duration::from_secs(expires_in),
        )?)
        .await?;

    Ok(presigned.uri().to_string())
}

pub async fn commit_img(
    db: &impl ConnectionTrait,

    room_id: Uuid,
    user_id: Uuid,
    title: Option<String>,
    object_key: String,
) -> Result<Uuid, RepositoryErr> {
    Ok(images::ActiveModel {
        title: sea_orm::ActiveValue::Set(title),
        room_id: sea_orm::ActiveValue::Set(room_id),
        created_at: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
        deleted_at: sea_orm::ActiveValue::Set(None as Option<NaiveDateTime>),
        user_id: sea_orm::ActiveValue::Set(user_id),
        object_key: sea_orm::ActiveValue::Set(object_key),
        score: sea_orm::ActiveValue::Set(0),
        ..Default::default()
    }
    .insert(db)
    .await?
    .id)
}

pub async fn update_commit_img_status(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    user_id: &Uuid,
    timeout: usize,
) -> Result<(), RepositoryErr> {
    let post_comment_tag = generate_redis_post_img_tag(user_id);

    let _: String = redis::cmd("SET")
        .arg(post_comment_tag)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(timeout)
        .query_async(&mut **conn)
        .await?;

    Ok(())
}

pub async fn check_if_his_img_waits_enough(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    user_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    let post_img_tag = generate_redis_post_img_tag(user_id);
    Ok(conn
        .get::<String, Option<String>>(post_img_tag)
        .await?
        .is_none())
}

pub async fn check_if_his_comment_waits_enough(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    user_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    let post_comment_tag = generate_redis_post_comment_tag(user_id);
    Ok(conn
        .get::<String, Option<String>>(post_comment_tag)
        .await?
        .is_none())
}

pub async fn post_comment(
    db: &impl ConnectionTrait,

    room_id: Uuid,
    user_id: Uuid,
    display_name: Option<String>,
    content: String,
) -> Result<Uuid, RepositoryErr> {
    Ok(comment::ActiveModel {
        room_id: sea_orm::ActiveValue::Set(room_id),
        user_id: sea_orm::ActiveValue::Set(user_id),
        content: sea_orm::ActiveValue::Set(content),
        display_name: sea_orm::ActiveValue::Set(display_name),
        created_at: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
        deleted_at: sea_orm::ActiveValue::Set(None as Option<NaiveDateTime>),
        ..Default::default()
    }
    .insert(db)
    .await?
    .id)
}

pub async fn update_post_comment_status(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    user_id: &Uuid,
    timeout: usize,
) -> Result<(), RepositoryErr> {
    let post_comment_tag = generate_redis_post_comment_tag(user_id);

    let _: String = redis::cmd("SET")
        .arg(post_comment_tag)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(timeout)
        .query_async(&mut **conn)
        .await?;

    Ok(())
}

pub async fn vote_good(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    img_id: Uuid,
    is_good: bool,
) -> Result<(), RepositoryErr> {
    if let Some(m) = image_vote::Entity::find()
        .filter(image_vote::Column::UserId.eq(user_id.to_string()))
        .filter(image_vote::Column::ImageId.eq(img_id.to_string()))
        .one(db)
        .await?
    {
        let mut active_model: image_vote::ActiveModel = m.into();
        active_model.is_good = sea_orm::ActiveValue::Set(is_good);

        active_model.update(db).await?;
    } else {
        image_vote::ActiveModel {
            image_id: sea_orm::ActiveValue::Set(img_id),
            user_id: sea_orm::ActiveValue::Set(user_id),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
            is_good: sea_orm::ActiveValue::Set(is_good),
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

pub async fn delete_img(db: &impl ConnectionTrait, img_id: Uuid) -> Result<(), RepositoryErr> {
    if let Some(m) = images::Entity::find()
        .filter(images::Column::Id.eq(img_id.to_string()))
        .one(db)
        .await?
    {
        let mut active_model: images::ActiveModel = m.into();
        active_model.deleted_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().naive_utc()));

        active_model.update(db).await?;
    }

    Ok(())
}

pub async fn delete_comment(
    db: &impl ConnectionTrait,
    comment_id: &Uuid,
) -> Result<(), RepositoryErr> {
    if let Some(m) = comment::Entity::find()
        .filter(comment::Column::Id.eq(comment_id.to_string()))
        .one(db)
        .await?
    {
        let mut active_model: comment::ActiveModel = m.into();
        active_model.deleted_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().naive_utc()));

        active_model.update(db).await?;
    }

    Ok(())
}

pub async fn check_if_ban_tag_exists(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    tag: &str,
) -> Result<bool, RepositoryErr> {
    Ok(conn.get::<&str, Option<String>>(tag).await?.is_some())
}

// if room id not specified, target will be banned from all room
pub async fn ban_user(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    room_id: &Uuid,
    user_identifier: &str,
) -> Result<(), RepositoryErr> {
    Ok(conn
        .hset::<String, &str, &str, _>(room_id.to_string(), user_identifier, "1")
        .await?)
}

pub async fn resolve_ban(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    room_id: &Uuid,
    user_identifier: &str,
) -> Result<(), RepositoryErr> {
    Ok(conn
        .hdel::<String, &str, _>(room_id.to_string(), user_identifier)
        .await?)
}

pub async fn check_if_he_is_banned(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    room_id: &Uuid,
    user_identifier: &str,
) -> Result<bool, RepositoryErr> {
    Ok(conn
        .hget::<String, &str, Option<String>>(room_id.to_string(), user_identifier)
        .await?
        .is_some())
}

pub async fn get_all_banned_users(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    room_id: Uuid,
) -> Result<Option<Vec<String>>, RepositoryErr> {
    Ok(
        if let Some(v) = conn
            .hgetall::<String, Option<HashMap<String, String>>>(room_id.to_string())
            .await?
        {
            Some(v.into_keys().collect())
        } else {
            None
        },
    )
}

fn generate_redis_presigned_url_save(presigned_url: &str, user_id: &Uuid) -> String {
    format!(
        "presigned:{}:{:x}",
        user_id,
        sha2::Sha256::digest(presigned_url.as_bytes())
    )
}

pub async fn add_valid_presigned_url(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    user_id: &Uuid,
    presigned_url: &str,
    object_key: &str,
    expires_in: u64,
) -> Result<(), RepositoryErr> {
    let redis_presigned_tag = generate_redis_presigned_url_save(presigned_url, user_id);
    let _: String = redis::cmd("SET")
        .arg(redis_presigned_tag)
        .arg(object_key)
        .arg("NX")
        .arg("EX")
        .arg(expires_in)
        .query_async(&mut **conn)
        .await?;
    Ok(())
}

pub async fn get_object_key(
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    user_id: &Uuid,
    presigned_url: &str,
) -> Result<Option<String>, RepositoryErr> {
    let redis_presigned_tag = generate_redis_presigned_url_save(presigned_url, user_id);
    Ok(conn
        .get::<String, Option<String>>(redis_presigned_tag)
        .await?)
}

pub async fn check_if_room_has_img(
    db: &impl ConnectionTrait,
    img_id: &Uuid,
    room_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    if let Some(m) = images::Entity::find()
        .filter(images::Column::Id.eq(img_id.to_string()))
        .one(db)
        .await?
    {
        return Ok(m.room_id == *room_id);
    };

    Ok(false)
}

pub async fn check_if_room_has_comment(
    db: &impl ConnectionTrait,
    comment_id: &Uuid,
    room_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    if let Some(m) = comment::Entity::find()
        .filter(comment::Column::Id.eq(comment_id.to_string()))
        .one(db)
        .await?
    {
        return Ok(m.room_id == *room_id);
    };

    Ok(false)
}

pub async fn check_if_img_vote_exists(
    db: &impl ConnectionTrait,
    user_id: &Uuid,
    img_id: &Uuid,
) -> Result<Option<image_vote::Model>, RepositoryErr> {
    Ok(image_vote::Entity::find()
        .join(
            sea_orm::JoinType::InnerJoin,
            image_vote::Relation::Images.def().rev(),
        )
        .filter(images::Column::DeletedAt.eq(None as Option<NaiveDateTime>))
        .filter(image_vote::Column::UserId.eq(user_id.to_string()))
        .filter(image_vote::Column::ImageId.eq(img_id.to_string()))
        .one(db)
        .await?)
}

pub async fn upsert_img_vote(
    db: &impl ConnectionTrait,
    model: Option<image_vote::Model>,
    user_id: Uuid,
    img_id: Uuid,
    is_good: bool,
) -> Result<(), RepositoryErr> {
    match model {
        None => {
            image_vote::ActiveModel {
                image_id: sea_orm::ActiveValue::Set(img_id),
                is_good: sea_orm::ActiveValue::Set(is_good),
                created_at: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
                user_id: sea_orm::ActiveValue::Set(user_id),
            }
            .insert(db)
            .await?;
        }
        Some(m) => {
            let mut active_model: image_vote::ActiveModel = m.into();
            active_model.is_good = sea_orm::ActiveValue::Set(is_good);
            active_model.created_at = sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc());

            active_model.update(db).await?;
        }
    }

    Ok(())
}

pub async fn find_user_id_with_img_id(
    db: &impl ConnectionTrait,
    img_id: &Uuid,
) -> Result<Option<Uuid>, RepositoryErr> {
    Ok(
        if let Some(m) = user::Entity::find()
            .join(
                sea_orm::JoinType::InnerJoin,
                user::Relation::Images.def().rev(),
            )
            .filter(images::Column::Id.eq(img_id.to_string()))
            .one(db)
            .await?
        {
            Some(m.id)
        } else {
            None
        },
    )
}

pub async fn delete_room(db: &impl ConnectionTrait, room_id: &Uuid) -> Result<(), RepositoryErr> {
    if let Some(room) = room::Entity::find()
        .filter(room::Column::Id.eq(room_id.to_string()))
        .one(db)
        .await?
    {
        if room.deleted_at.is_none() {
            let mut active_value: room::ActiveModel = room.into();
            active_value.deleted_at =
                sea_orm::ActiveValue::Set(Some(chrono::Utc::now().naive_utc()));
            active_value.update(db).await?;
        }
    }

    Ok(())
}
