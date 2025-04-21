use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;

use cogni_core::error::MemoryError;
use cogni_core::traits::memory::{MemoryEntry, MemoryQuery, MemoryStore, Role, SessionId};

use memory::redis::RedisMemory;
use memory::sqlite::{SqliteConfig, SqliteStore};

async fn setup_sqlite_store() -> SqliteStore {
    let config = SqliteConfig::new("sqlite::memory:");
    SqliteStore::new(config)
        .await
        .expect("Failed to create SQLite store")
}

async fn setup_redis_store() -> RedisMemory {
    // Use a test Redis URL and prefix; adjust if needed for your test environment
    let url = "redis://127.0.0.1/";
    let prefix = "test_query_history:";
    RedisMemory::new(url, prefix).expect("Failed to create Redis store")
}

fn create_test_entries() -> Vec<MemoryEntry> {
    let now = OffsetDateTime::now_utc();
    vec![
        MemoryEntry {
            role: Role::User,
            content: "Hello world".to_string(),
            timestamp: now,
        },
        MemoryEntry {
            role: Role::Assistant,
            content: "Hi there!".to_string(),
            timestamp: now + Duration::from_secs(10).into(),
        },
        MemoryEntry {
            role: Role::System,
            content: "System message".to_string(),
            timestamp: now + Duration::from_secs(20).into(),
        },
        MemoryEntry {
            role: Role::User,
            content: "Another user message".to_string(),
            timestamp: now + Duration::from_secs(30).into(),
        },
    ]
}

async fn insert_entries(store: &impl MemoryStore, session: &SessionId, entries: &[MemoryEntry]) {
    for entry in entries {
        store
            .save(session, entry.clone())
            .await
            .expect("Failed to save entry");
    }
}

fn assert_results_equal(
    res_sqlite: &Result<Vec<MemoryEntry>, MemoryError>,
    res_redis: &Result<Vec<MemoryEntry>, MemoryError>,
) {
    match (res_sqlite, res_redis) {
        (Ok(vec_sqlite), Ok(vec_redis)) => {
            assert_eq!(vec_sqlite.len(), vec_redis.len(), "Result lengths differ");
            for (a, b) in vec_sqlite.iter().zip(vec_redis.iter()) {
                assert_eq!(a.role, b.role, "Roles differ");
                assert_eq!(a.content, b.content, "Contents differ");
                assert_eq!(a.timestamp, b.timestamp, "Timestamps differ");
            }
        }
        (Err(e_sqlite), Err(e_redis)) => {
            // Compare error variants or messages if needed
            assert_eq!(
                format!("{:?}", e_sqlite),
                format!("{:?}", e_redis),
                "Errors differ"
            );
        }
        _ => panic!("One result is Ok and the other is Err"),
    }
}

#[tokio::test]
async fn test_query_history_consistency() {
    let session = SessionId::new("test_session");

    let sqlite_store = setup_sqlite_store().await;
    let redis_store = setup_redis_store().await;

    let entries = create_test_entries();

    insert_entries(&sqlite_store, &session, &entries).await;
    insert_entries(&redis_store, &session, &entries).await;

    // Allow some time for Redis to persist data if needed
    sleep(Duration::from_millis(100)).await;

    // Test cases with various query parameters
    let test_cases = vec![
        MemoryQuery {
            session: session.clone(),
            offset: None,
            limit: None,
            start_time: None,
            end_time: None,
            role: None,
            content_substring: None,
        },
        MemoryQuery {
            session: session.clone(),
            offset: Some(1),
            limit: Some(2),
            start_time: None,
            end_time: None,
            role: None,
            content_substring: None,
        },
        MemoryQuery {
            session: session.clone(),
            offset: None,
            limit: None,
            start_time: Some(entries[1].timestamp),
            end_time: Some(entries[2].timestamp),
            role: None,
            content_substring: None,
        },
        MemoryQuery {
            session: session.clone(),
            offset: None,
            limit: None,
            start_time: None,
            end_time: None,
            role: Some(Role::User),
            content_substring: None,
        },
        MemoryQuery {
            session: session.clone(),
            offset: None,
            limit: None,
            start_time: None,
            end_time: None,
            role: None,
            content_substring: Some("user".to_string()),
        },
    ];

    for query in test_cases {
        let res_sqlite = sqlite_store.query_history(query.clone()).await;
        let res_redis = redis_store.query_history(query).await;
        assert_results_equal(&res_sqlite, &res_redis);
    }
}

#[tokio::test]
async fn test_query_history_error_handling() {
    let session = SessionId::new("nonexistent_session");

    let sqlite_store = setup_sqlite_store().await;
    let redis_store = setup_redis_store().await;

    // Query history for a session that does not exist should return Ok with empty vec or consistent error
    let query = MemoryQuery {
        session: session.clone(),
        offset: None,
        limit: None,
        start_time: None,
        end_time: None,
        role: None,
        content_substring: None,
    };

    let res_sqlite = sqlite_store.query_history(query.clone()).await;
    let res_redis = redis_store.query_history(query).await;

    assert_results_equal(&res_sqlite, &res_redis);
}
