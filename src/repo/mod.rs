mod core;
mod in_memory;
mod sqlite;

// TODO: repo config enum
pub use self::core::Repo;
pub use self::in_memory::InMemoryRepo;
pub use self::sqlite::SqliteRepo;
