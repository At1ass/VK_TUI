//! VK API method implementations organized by namespace

pub mod account;
pub mod friends;
pub mod longpoll;
pub mod messages;
pub mod users;

pub use account::AccountApi;
pub use friends::FriendsApi;
pub use longpoll::LongPollApi;
pub use messages::MessagesApi;
pub use users::UsersApi;
