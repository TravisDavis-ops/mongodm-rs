//! MongODM
//! =======
//!
//! A thin ODM layer for MongoDB built upon the [official Rust driver](https://github.com/mongodb/mongo-rust-driver).
//!
//! Main features:
//!
//! - A stronger API leveraging Rust type system
//! - Data structure models are defined using the well-known [`serde`](https://github.com/serde-rs/serde) serialization framework
//! - Index support on top of the `Database::run_command` (index management is currently not implemented in the underlying driver)
//! - Indexes synchronization
//! - Additional compile-time checks for queries using macros and type associated to mongo operators (eg: `And` instead of "$and")
//!
//! ## Example
//!
//! ```ignore
//! # async fn demo() -> Result<(), mongodb::error::Error> {
//! use mongodm::{ToRepository, Model, CollectionConfig, Indexes, Index, IndexOption, sync_indexes};
//! use mongodm::mongo::{Client, options::ClientOptions, bson::doc};
//! use serde::{Serialize, Deserialize};
//! use std::borrow::Cow;
//! // field! is used to make sure at compile time that some field exists in a given structure
//! use mongodm::field;
//!
//! struct UserCollConf;
//!
//! impl CollectionConfig for UserCollConf {
//!     fn collection_name() -> &'static str {
//!         "user"
//!     }
//!
//!     fn indexes() -> Indexes {
//!         Indexes::new()
//!             .with(Index::new("username").with_option(IndexOption::Unique))
//!             .with(Index::new(field!(last_seen in User))) // field! macro can be used as well
//!     }
//! }
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! struct User {
//!     username: String,
//!     last_seen: i64,
//! }
//!
//! impl Model for User {
//!     type CollConf = UserCollConf;
//! }
//!
//! let client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
//! let client = Client::with_options(client_options)?;
//! let db = client.database("mongodm_wayk_demo");
//!
//! sync_indexes::<UserCollConf>(&db).await?;
//! // indexes are now synced in backend for user collection
//!
//! let repository = db.repository::<User>(); // method provided by `ToRepository` trait
//!
//! let user = User {
//!     username: String::from("David"),
//!     last_seen: 1000,
//! };
//! repository.insert_one(&user, None).await?;
//!
//! // We make sure at compile time that `username` is a field of `User`
//! let fetched_user = repository.find_one(doc! { field!(username in User): "David" }, None).await?;
//! assert!(fetched_user.is_some());
//! assert_eq!(fetched_user.unwrap(), user);
//!
//! // f! is a shorter version of field!
//! use mongodm::f;
//! repository.find_one(doc! { f!(username in User): "David" }, None).await?.unwrap();
//!
//! // With static operators for queries (prevent invalid queries due to typos)
//! use mongodm::operator::*;
//! repository.find_one(
//!     doc! { And: [
//!         { f!(username in User): "David" },
//!         { f!(last_seen in User): { GreaterThan: 500 } },
//!     ] },
//!     None
//! ).await?.unwrap();
//! # Ok(())
//! # }
//! # let mut rt = tokio::runtime::Runtime::new().unwrap();
//! # rt.block_on(demo());
//! ```

#[macro_use]
#[cfg(test)]
extern crate pretty_assertions;

mod macros;

pub mod cursor;
pub mod index;
pub mod operator;
pub mod repository;

pub use cursor::ModelCursor;
pub use index::{sync_indexes, Index, IndexOption, Indexes, SortOrder};
pub use repository::Repository;

// Re-export mongodb
pub use mongodb as mongo;
// Re-export bson
pub use mongodb::bson;

/// Associate a collection configuration
pub trait Model: serde::ser::Serialize + serde::de::DeserializeOwned {
    type CollConf: CollectionConfig;
}

/// Define collection name, configuration and associated indexes.
pub trait CollectionConfig {
    /// Collection name to use when creating a `mongodb::Collection` instance.
    fn collection_name() -> &'static str;

    /// `mongodb::options::CollectionOptions` to be used when creating a `mongodb::Collection` instance.
    ///
    /// This method has a default implementation returning `None`.
    /// In such case configuration is defined by the `mongodb::Database` used on `Repository` creation.
    fn collection_options() -> Option<mongodb::options::CollectionOptions> {
        None
    }

    /// Configure how indexes should be created and synchronized for the associated collection.
    ///
    /// This method has a default implementation returning no index (only special `_id` index will be present).
    fn indexes() -> index::Indexes {
        index::Indexes::default()
    }
}

/// Utilities methods to get a `Repository`. Implemented for `mongodb::Database`.
pub trait ToRepository {
    /// Shorthand for `Repository::<Model>::new`.
    fn repository<M: Model>(&self) -> Repository<M>;

    /// Shorthand for `Repository::<Model>::new_with_options`.
    fn repository_with_options<M: Model>(
        &self,
        options: mongodb::options::CollectionOptions,
    ) -> Repository<M>;
}
#[cfg(featurn = "sync-runtime")]
impl ToRepository for mongodb::sync::Database {
    fn repository<M: Model>(&self) -> Repository<M> {
        Repository::new(self.clone())
    }

    fn repository_with_options<M: Model>(
        &self,
        options: mongodb::options::CollectionOptions,
    ) -> Repository<M> {
        Repository::new_with_options(self.clone(), options)
    }
}
#[cfg(feature = "tokio-runtime")]
impl ToRepository for mongodb::Database {
    fn repository<M: Model>(&self) -> Repository<M> {
        Repository::new(self.clone())
    }

    fn repository_with_options<M: Model>(
        &self,
        options: mongodb::options::CollectionOptions,
    ) -> Repository<M> {
        Repository::new_with_options(self.clone(), options)
    }
}
