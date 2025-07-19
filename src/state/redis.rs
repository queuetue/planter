use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type RedisClient = Arc<Mutex<redis::aio::MultiplexedConnection>>;

pub async fn connect(redis_url: &str) -> Result<RedisClient, redis::RedisError> {
    let client = redis::Client::open(redis_url)?;
    let conn = client.get_multiplexed_async_connection().await?;
    Ok(Arc::new(Mutex::new(conn)))
}

pub async fn set_json<T: serde::Serialize + ?Sized>(
    client: &RedisClient,
    key: &str,
    value: &T,
) -> redis::RedisResult<()> {
    let json = serde_json::to_string(value)
        .map_err(|e| redis::RedisError::from((redis::ErrorKind::InvalidClientConfig, "JSON serialization failed", e.to_string())))?;
    client.lock().await.set(key, json).await
}

pub async fn get_json<T: for<'de> serde::Deserialize<'de>>(
    client: &RedisClient,
    key: &str,
) -> redis::RedisResult<Option<T>> {
    let data: Option<String> = client.lock().await.get(key).await?;
    Ok(data.and_then(|s| serde_json::from_str(&s).ok()))
}
