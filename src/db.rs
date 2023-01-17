// use std::num::NonZeroUsize;
// use std::sync::Arc;
// use lru::LruCache;
// use rusqlite::{params, Connection, Error};
// use serenity::model::id::{ChannelId, GuildId, MessageId, UserId};
// use serenity::model::prelude::{Message, User};
// use serenity::model::Timestamp;
// use tokio::sync::{RwLock, RwLockWriteGuard};
// use tokio_rusqlite::Connection as AsyncConnection;
//
// // struct based db that uses a cache to store the data
//
// pub fn get_connection() -> Result<Connection, Error> {
//     Ok(Connection::open("db.sqlite")?)
// }
//
// pub struct DB {
//     pub conn: AsyncConnection,
//     pub guilds: RwLock<LruCache<GuildId, RankGuild>>, // cache of guilds
//     pub channels: RwLock<LruCache<ChannelId, RankChannel>>, // cache of channels
//     pub users: RwLock<LruCache<UserId, RankUser>>,    // cache of users
//     pub messages: RwLock<LruCache<MessageId, RankMessage>>, // cache of messages
// }
//
// impl DB {
//     pub fn new() -> DB {
//         DB {
//             conn: AsyncConnection::open("db.sqlite").unwrap(),
//             guilds: RwLock::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
//             channels: RwLock::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
//             users: RwLock::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
//             messages: RwLock::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
//         }
//     }
//
//     pub fn init_db(conn: &mut Connection) -> Result<(), Error> {
//         conn.execute(
//             "CREATE TABLE IF NOT EXISTS guilds (
//                 id INTEGER PRIMARY KEY,
//                 snowflake INTEGER NOT NULL,
//                 name TEXT NOT NULL,
//                 score INTEGER NOT NULL,
//                 message_count INTEGER NOT NULL,
//                 user_count INTEGER NOT NULL
//             )",
//             params![],
//         )?;
//         conn.execute(
//             "CREATE TABLE IF NOT EXISTS channels (
//                 id INTEGER PRIMARY KEY,
//                 snowflake INTEGER NOT NULL,
//                 name TEXT NOT NULL,
//                 score INTEGER NOT NULL,
//                 message_count INTEGER NOT NULL,
//                 guild INTEGER NOT NULL,
//                 FOREIGN KEY(guild) REFERENCES guilds(id)
//             )",
//             params![],
//         )?;
//         conn.execute(
//             "CREATE TABLE IF NOT EXISTS users (
//                 id INTEGER PRIMARY KEY,
//                 snowflake INTEGER NOT NULL,
//                 message_count INTEGER NOT NULL,
//                 score INTEGER NOT NULL,
//                 guild INTEGER NOT NULL,
//                 FOREIGN KEY(guild) REFERENCES guilds(id)
//             )",
//             params![],
//         )?;
//         conn.execute(
//             "CREATE TABLE IF NOT EXISTS messages (
//                 id INTEGER PRIMARY KEY,
//                 snowflake INTEGER NOT NULL,
//                 content TEXT NOT NULL,
//                 score INTEGER NOT NULL,
//                 replys_to INTEGER,
//                 channel INTEGER NOT NULL,
//                 user INTEGER NOT NULL,
//                 FOREIGN KEY(replys_to) REFERENCES messages(id),
//                 FOREIGN KEY(channel) REFERENCES channels(id),
//                 FOREIGN KEY(user) REFERENCES users(id)
//             )",
//             params![],
//         )?;
//         Ok(())
//     }
// }
//
// #[derive(Clone)]
// pub struct RankGuild {
//     pub id: Option<u64>,            // id of the guild in the database (stored in db)
//     pub snowflake: Option<GuildId>, // snowflake of the guild (stored in db)
//     pub name: Option<String>,       // name of the guild (stored in db)
//     pub score: Option<i32>,         // total score of the guild (stored in db)
//     pub message_count: Option<i32>, // how many messages are sent in the guild (stored in db)
//     pub messages: Option<Vec<RankMessage>>, // messages sent by the user (obtained from messages table)
//     pub user_count: Option<i32>,            // how many users are in the guild (stored in db)
//     pub rank: Option<i32>,                  // rank of the guild (calculated from score)
//     pub users: Option<Vec<RankUser>>,       // users in the guild (obtained from users table)
//     pub channels: Option<Vec<RankChannel>>, // channels in the guild (obtained from channels table)
// }
//
// impl RankGuild {
//     pub fn new(snowflake: GuildId) -> RankGuild {
//         RankGuild {
//             id: None,
//             snowflake: Some(snowflake),
//             name: None,
//             score: None,
//             message_count: None,
//             messages: None,
//             user_count: None,
//             rank: None,
//             users: None,
//             channels: None,
//         }
//     }
//
//     pub fn update(&mut self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<GuildId, RankGuild>>, use_cache:bool) -> Result<(), Error> {
//         // get latest data from db
//
//         if use_cache {
//             if let Some(guild) = cache.get(&self.snowflake.unwrap()) {
//                 self.id = guild.id;
//                 self.name = guild.name.clone();
//                 self.score = guild.score;
//                 self.message_count = guild.message_count;
//                 self.messages = guild.messages.clone();
//                 self.user_count = guild.user_count;
//                 self.rank = guild.rank;
//                 self.users = guild.users.clone();
//                 self.channels = guild.channels.clone();
//                 Ok(())
//             }
//         }
//
//         log::debug!("executing: 'SELECT * FROM guilds WHERE snowflake = {}'", self.snowflake.unwrap().0);
//
//         let mut stmt = conn.prepare("SELECT * FROM guilds WHERE snowflake = ?1")?;
//         let mut rows = stmt.query(params![self.snowflake.unwrap().0])?;
//         let row = rows.next().unwrap()?;
//         self.id = Some(row.get(0)?);
//         self.name = Some(row.get(2)?);
//         self.score = Some(row.get(3)?);
//         self.message_count = Some(row.get(4)?);
//         self.user_count = Some(row.get(5)?);
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         Ok(())
//     }
//
//     pub fn get_messages(&mut self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<GuildId, RankGuild>>, use_cache:bool) -> Result<(), Error> {
//
//         if use_cache {
//             if let Some(guild) = cache.get(&self.snowflake.unwrap()) {
//                 self.messages = guild.messages.clone();
//                 Ok(())
//             }
//         }
//
//         // get messages from db
//         log::debug!("executing: 'SELECT * FROM messages WHERE guild = {}'", self.id.unwrap());
//
//         let mut stmt = conn.prepare("SELECT * FROM messages WHERE guild = ?1")?;
//         let mut rows = stmt.query(params![self.id.unwrap()])?;
//         let mut messages = Vec::new();
//         while let Some(row) = rows.next().unwrap() {
//             let mut message = RankMessage::new(row.get(1)?);
//             message.id = Some(row.get(0)?);
//             message.content = Some(row.get(2)?);
//             message.score = Some(row.get(3)?);
//             message.replys_to = Some(Box::new(RankMessage::new(row.get(4)?)));
//             message.channel = Some(RankChannel::new(row.get(5)?));
//             message.author = Some(RankUser::new(row.get(6)?));
//             messages.push(message);
//         }
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         self.messages = Some(messages);
//         Ok(())
//     }
//
//     pub fn get_users(&mut self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<GuildId, RankGuild>>, use_cache:bool) -> Result<(), Error> {
//
//         if use_cache {
//             if let Some(guild) = cache.get(&self.snowflake.unwrap()) {
//                 self.users = guild.users.clone();
//                 Ok(())
//             }
//         }
//
//         // get users from db
//         log::debug!("executing: 'SELECT * FROM users WHERE guild = {}'", self.id.unwrap());
//
//         let mut stmt = conn.prepare("SELECT * FROM users WHERE guild = ?1")?;
//         let mut rows = stmt.query(params![self.id.unwrap()])?;
//         let mut users = Vec::new();
//         while let Some(row) = rows.next().unwrap() {
//             let mut user = RankUser::new(row.get(1)?);
//             user.id = Some(row.get(0)?);
//             user.message_count = Some(row.get(2)?);
//             user.score = Some(row.get(3)?);
//             user.guild = Some(RankGuild::new(row.get(4)?));
//             users.push(user);
//         }
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         self.users = Some(users);
//         Ok(())
//     }
//
//     pub fn get_channels(&mut self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<GuildId, RankGuild>>, use_cache:bool) -> Result<(), Error> {
//
//         if use_cache {
//             if let Some(guild) = cache.get(&self.snowflake.unwrap()) {
//                 self.channels = guild.channels.clone();
//                 Ok(())
//             }
//         }
//
//         // get channels from db
//         log::debug!("executing: 'SELECT * FROM channels WHERE guild = {}'", self.id.unwrap());
//
//         let mut stmt = conn.prepare("SELECT * FROM channels WHERE guild = ?1")?;
//         let mut rows = stmt.query(params![self.id.unwrap()])?;
//         let mut channels = Vec::new();
//         while let Some(row) = rows.next().unwrap() {
//             let mut channel = RankChannel::new(row.get(1)?);
//             channel.id = Some(row.get(0)?);
//             channel.name = Some(row.get(2)?);
//             channel.score = Some(row.get(3)?);
//             channel.message_count = Some(row.get(4)?);
//             channel.guild = Some(RankGuild::new(row.get(5)?));
//             channels.push(channel);
//         }
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         self.channels = Some(channels);
//         Ok(())
//     }
//
//     pub fn save(&self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<GuildId, RankGuild>>) -> Result<(), Error> {
//         // save guild to db
//         log::debug!("executing: 'INSERT INTO guilds (snowflake, name, score, message_count, user_count) VALUES ({}, {}, {}, {}, {})'", self.snowflake.unwrap().0, self.name.clone().unwrap(), self.score.clone().unwrap(), self.message_count.clone().unwrap(), self.user_count.clone().unwrap());
//
//         let mut stmt = conn.prepare("INSERT INTO guilds (snowflake, name, score, message_count, user_count) VALUES (?1, ?2, ?3, ?4, ?5)")?;
//         stmt.execute(params![self.snowflake.unwrap().0, self.name.clone().unwrap(), self.score.clone().unwrap(), self.message_count.clone().unwrap(), self.user_count.clone().unwrap()])?;
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         Ok(())
//     }
// }
//
// #[derive(Clone)]
// pub struct RankChannel {
//     pub id: Option<u64>, // id of the channel in the database (stored in db)
//     pub snowflake: Option<ChannelId>, // snowflake of the channel (stored in db)
//     pub name: Option<String>, // name of the channel (stored in db)
//     pub score: Option<i32>, // total score of the channel (stored in db)
//     pub message_count: Option<i32>, // how many messages are sent in the channel (stored in db)
//     pub messages: Option<Vec<RankMessage>>, // messages in channel (obtained from messages table)
//     pub rank: Option<i32>, // rank of the channel (calculated from score)
//     pub guild: Option<RankGuild>, // guild the channel is in (foreign key)
// }
//
// impl RankChannel {
//     pub fn new(snowflake: ChannelId) -> RankChannel {
//         RankChannel {
//             id: None,
//             snowflake: Some(snowflake),
//             name: None,
//             score: None,
//             message_count: None,
//             messages: None,
//             rank: None,
//             guild: None,
//         }
//     }
//
//     pub fn get_messages(&mut self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<ChannelId, RankChannel>>, use_cache:bool) -> Result<(), Error> {
//
//         if use_cache {
//             if let Some(channel) = cache.get(&self.snowflake.unwrap()) {
//                 self.messages = channel.messages.clone();
//                 Ok(())
//             }
//         }
//
//         // get messages from db
//         log::debug!("executing: 'SELECT * FROM messages WHERE channel = {}'", self.id.unwrap());
//
//         let mut stmt = conn.prepare("SELECT * FROM messages WHERE channel = ?1")?;
//         let mut rows = stmt.query(params![self.id.unwrap()])?;
//         let mut messages = Vec::new();
//         while let Some(row) = rows.next().unwrap() {
//             let mut message = RankMessage::new(row.get(1)?);
//             message.id = Some(row.get(0)?);
//             message.content = Some(row.get(2)?);
//             message.score = Some(row.get(3)?);
//             message.replys_to = Some(Box::new(RankMessage::new(row.get(4)?)));
//             message.channel = Some(RankChannel::new(row.get(5)?));
//             message.author = Some(RankUser::new(row.get(6)?));
//             messages.push(message);
//         }
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         self.messages = Some(messages);
//         Ok(())
//     }
//
//     pub fn save(&self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<ChannelId, RankChannel>>) -> Result<(), Error> {
//         // save channel to db
//         log::debug!("executing: 'INSERT INTO channels (snowflake, name, score, message_count, guild) VALUES ({}, {}, {}, {}, {})'", self.snowflake.unwrap().0, self.name.clone().unwrap(), self.score.clone().unwrap(), self.message_count.clone().unwrap(), self.guild.clone().unwrap().id.unwrap());
//
//         let mut stmt = conn.prepare("INSERT INTO channels (snowflake, name, score, message_count, guild) VALUES (?1, ?2, ?3, ?4, ?5)")?;
//         stmt.execute(params![self.snowflake.unwrap().0, self.name.clone().unwrap(), self.score.clone().unwrap(), self.message_count.clone().unwrap(), self.guild.clone().unwrap().id.unwrap()])?;
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         Ok(())
//     }
// }
//
// #[derive(Clone)]
// pub struct RankUser {
//     pub id: Option<u32>,            // id of the user in the database (stored in db)
//     pub snowflake: Option<UserId>,  // snowflake of the user (stored in db)
//     pub message_count: Option<i32>, // messages sent by the user (stored in db)
//     pub score: Option<i32>,         // score of the user (stored in db)
//     pub last_message: Option<Timestamp>, // timestamp of the last message sent by the user (not stored in db)
//     pub rank: Option<i32>,               // rank of the user (not stored in db)
//     pub messages: Option<Vec<RankMessage>>, // messages sent by the user (obtained from messages table)
//     pub guild: Option<RankGuild>,           // guild the user is in (foreign key)
//     pub stats: Option<Box<RankUserStats>>,       // stats of the user  (not stored in db)
// }
//
// impl RankUser {
//     pub fn new(snowflake: UserId) -> RankUser {
//         RankUser {
//             id: None,
//             snowflake: Some(snowflake),
//             message_count: None,
//             score: None,
//             last_message: None,
//             rank: None,
//             messages: None,
//             guild: None,
//             stats: None,
//         }
//     }
//
//     pub async fn update(&mut self, conn: &AsyncConnection, mut cache: RwLockWriteGuard<LruCache<UserId, RankUser>>, use_cache:bool) -> Result<(), Error> {
//
//         if use_cache {
//             if let Some(user) = cache.get(&self.snowflake.unwrap()) {
//                 self.id = user.id;
//                 self.snowflake = user.snowflake;
//                 self.message_count = user.message_count;
//                 self.score = user.score;
//                 self.last_message = user.last_message;
//                 self.rank = user.rank;
//                 self.messages = user.messages.clone();
//                 self.guild = user.guild.clone();
//                 self.stats = user.stats.clone();
//                 Ok(())
//             }
//         }
//
//         // get latest data from db
//         log::debug!("executing: 'SELECT * FROM users WHERE snowflake = {}'", self.snowflake.unwrap().0);
//
//         conn.call(|conn| {
//             let mut stmt = conn.prepare("SELECT * FROM users WHERE snowflake = ?1")?;
//             let mut rows = stmt.query(params![self.snowflake.unwrap().0])?;
//             let row = rows.next().unwrap()?;
//             self.id = Some(row.get(0)?);
//             self.message_count = Some(row.get(2)?);
//             self.score = Some(row.get(3)?);
//             self.guild = Some(RankGuild::new(row.get(4)?));
//
//         }).await?;
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         Ok(())
//     }
//
//     pub fn get_messages(&mut self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<UserId, RankUser>>, use_cache: bool) -> Result<(), Error> {
//         if use_cache {
//             if let Some(user) = cache.get(&self.snowflake.unwrap()) {
//                 self.messages = user.messages.clone();
//                 return Ok(());
//             }
//         }
//
//         // get messages from db
//         log::debug!("executing: 'SELECT * FROM messages WHERE user = {}'", self.id.unwrap());
//
//         let mut stmt = conn.prepare("SELECT * FROM messages WHERE user = ?1")?;
//         let mut rows = stmt.query(params![self.id.unwrap()])?;
//         let mut messages = Vec::new();
//         while let Some(row) = rows.next().unwrap() {
//             let mut message = RankMessage::new(row.get(1)?);
//             message.id = Some(row.get(0)?);
//             message.content = Some(row.get(2)?);
//             message.score = Some(row.get(3)?);
//             message.replys_to = match row.get(4)? {
//                 Some(id) => Some(Box::new(RankMessage::new(id))),
//                 None => None,
//             };
//             message.channel = Some(RankChannel::new(row.get(5)?));
//             message.author = Some(RankUser::new(row.get(6)?));
//             messages.push(message);
//         }
//         self.messages = Some(messages);
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         Ok(())
//     }
//
//     pub fn save(&self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<UserId, RankUser>>) -> Result<(), Error> {
//         // save user to db
//         log::debug!("executing: 'INSERT INTO users (snowflake, user, message_count, score, last_message, guild) VALUES ({}, {}, {}, {}, {}, {})'", self.snowflake.unwrap().0, self.user.clone().unwrap().0, self.message_count.clone().unwrap(), self.score.clone().unwrap(), self.last_message.clone().unwrap().0, self.guild.clone().unwrap().id.unwrap());
//
//         let mut stmt = conn.prepare("INSERT INTO users (snowflake, user, message_count, score, last_message, guild) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
//         stmt.execute(params![self.snowflake.unwrap().0, self.user.clone().unwrap().0, self.message_count.clone().unwrap(), self.score.clone().unwrap(), self.last_message.clone().unwrap().0, self.guild.clone().unwrap().id.unwrap()])?;
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         Ok(())
//     }
// }
//
// pub struct RankMessage {
//     pub id: Option<i64>, // id of the message in the database (stored in db)
//     pub snowflake: Option<MessageId>, // id of the message in discord (contains timestamp) (stored in db)
//     pub content: Option<String>,      // content of the message (stored in db)
//     pub score: Option<i32>,           // score of the message (calculated from content)
//     pub replys_to: Option<Box<RankMessage>>, // the message this message is a reply to  (stored in db as id)
//     pub replys: Option<Vec<RankMessage>>, // messages that are replys to this message (obtained from messages table)
//     pub rank: Option<i32>,                // rank of the message (not stored in db)
//     pub author: Option<RankUser>,        // author of the message (foreign key)
//     pub channel: Option<RankChannel>,    // channel the message is in (foreign key)
// }
//
// impl RankMessage {
//     pub fn new(snowflake: MessageId) -> RankMessage {
//         RankMessage {
//             id: None,
//             snowflake: Some(snowflake),
//             content: None,
//             score: None,
//             replys_to: None,
//             replys: None,
//             rank: None,
//             author: None,
//             channel: None,
//         }
//     }
//
//     pub async fn update(mut self, conn: &AsyncConnection, mut cache: RwLockWriteGuard<LruCache<MessageId, RankMessage>>, use_cache: bool) -> Result<(), Error> {
//         if use_cache {
//             if let Some(message) = cache.get(&self.snowflake.unwrap()) {
//                 self.id = message.id;
//                 self.snowflake = message.snowflake;
//                 self.content = message.content.clone();
//                 self.score = message.score;
//                 self.replys_to = message.replys_to.clone();
//                 self.rank = message.rank;
//                 self.author = message.author.clone();
//                 self.channel = message.channel.clone();
//                 return Ok(());
//             }
//         }
//
//         // get latest data from db
//         log::debug!("executing: 'SELECT * FROM messages WHERE snowflake = {}'", self.snowflake.unwrap().0);
//
//         conn.call(|conn|{
//             let mut stmt = conn.prepare("SELECT * FROM messages WHERE snowflake = ?1")?;
//             let mut rows = stmt.query(params![self.snowflake.unwrap().0])?;
//
//             let row = rows.next().unwrap()?;
//             self.id = Some(row.get(0)?);
//             self.content = Some(row.get(2)?);
//             self.score = Some(row.get(3)?);
//             self.replys_to = match row.get(4)? {
//                 Some(id) => Some(Box::new(RankMessage::new(id))),
//                 None => None,
//             };
//             self.channel = Some(RankChannel::new(row.get(5)?));
//             self.author = Some(RankUser::new(row.get(6)?));
//         }).await?;
//
//
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         Ok(())
//     }
//
//     pub fn get_replys(&mut self, conn: &mut Connection, mut cache: RwLockWriteGuard<LruCache<MessageId, RankMessage>>, use_cache: bool) -> Result<(), Error> {
//         if use_cache {
//             if let Some(message) = cache.get(&self.snowflake.unwrap()) {
//                 self.replys = message.replys.clone();
//                 return Ok(());
//             }
//         }
//
//         // get replys from db
//         log::debug!("executing: 'SELECT * FROM messages WHERE replys_to = {}'", self.id.unwrap());
//
//         let mut stmt = conn.prepare("SELECT * FROM messages WHERE replys_to = ?1")?;
//         let mut rows = stmt.query(params![self.id.unwrap()])?;
//         let mut replys = Vec::new();
//         while let Some(row) = rows.next().unwrap() {
//             let mut message = RankMessage::new(row.get(1)?);
//             message.id = Some(row.get(0)?);
//             message.content = Some(row.get(2)?);
//             message.score = Some(row.get(3)?);
//             message.replys_to = Some(Box::new(RankMessage::new(self.snowflake.unwrap())));
//             message.channel = Some(RankChannel::new(row.get(5)?));
//             message.author = Some(RankUser::new(row.get(6)?));
//             replys.push(message);
//         }
//         self.replys = Some(replys);
//
//         cache.put(self.snowflake.unwrap(), self.clone());
//
//         Ok(())
//     }
// }
//
// pub struct RankUserStats {
//     // calculated from messages
//     pub most_active_day: Option<String>, // most active day of the user
//     pub most_active_hour: Option<String>, // most active hour of the user
//     pub most_active_channel: Option<String>, // most active channel of the user
//     pub best_message: Option<RankMessage>, // best message of the user
//     pub worst_message: Option<RankMessage>, // worst message of the user
//     pub most_active_day_count: Option<i32>, // how many messages the user sent on the most active day
//     pub most_active_hour_count: Option<i32>, // how many messages the user sent on the most active hour
//     pub most_active_channel_count: Option<i32>, // how many messages the user sent on the most active channel
//     pub average_message_score: Option<f32>,     // average score of the user's messages
//     pub average_message_length: Option<f32>,    // average length of the user's messages
//     pub average_message_word_count: Option<f32>, // average word count of the user's messages
//     pub favorite_word: Option<String>,          // favorite word of the user
//     pub favorite_word_count: Option<i32>,       // how many times the user used the favorite word
//     pub top_words: Option<Vec<(String, i32)>>,  // top 100 words of the user
// }
//
// // ----------------------------------------------------------------
// // messages
// // ----------------------------------------------------------------
//
// pub fn store_message(
//     conn: &Connection,
//     user_id: u64,
//     content: String,
//     timestamp: i64,
// ) -> Result<(), Error> {
//     let user_db_id = get_user_from_id(&conn, user_id)?.0;
//     log::debug!("executing: 'INSERT INTO discord_messages (author, content, timestamp) VALUES ({}, {}, {})'", user_db_id, content, timestamp);
//
//     let q = conn.execute(
//         "INSERT INTO discord_messages (author, content, timestamp)
//         VALUES (?1, ?2, ?3)",
//         params![user_db_id, content, timestamp],
//     );
//
//     match q {
//         Ok(_) => Ok(()),
//         Err(e) => Err(e),
//     };
//
//     Ok(())
// }
//
// pub fn get_messages_from_user(conn: &Connection, id: u64) -> Result<Vec<String>, Error> {
//     let mut stmt = conn.prepare("SELECT message FROM discord_messages WHERE author = ?1")?;
//     let messages = stmt.query_map([id], |row| row.get(0))?;
//     let mut messages_vec = Vec::new();
//     for message in messages {
//         messages_vec.push(message?);
//     }
//     Ok(messages_vec)
// }
//
// pub fn get_messages(conn: &Connection) -> Result<Vec<String>, Error> {
//     let mut stmt = conn.prepare("SELECT message FROM discord_messages")?;
//     let messages = stmt.query_map([], |row| row.get(0))?;
//     let mut messages_vec = Vec::new();
//     for message in messages {
//         messages_vec.push(message?);
//     }
//     Ok(messages_vec)
// }
//
// // ----------------------------------------------------------------
// // users
// // ----------------------------------------------------------------
//
// pub fn get_user_from_id(conn: &Connection, id: u64) -> Result<(u64, u64, String, u64, u64), Error> {
//     log::debug!(
//         "executing: 'SELECT * FROM discord_rank WHERE user_id = {}'",
//         id
//     );
//     let mut stmt = conn.prepare("SELECT * FROM discord_rank WHERE user_id = ?1");
//     match stmt {
//         Ok(mut stmt) => {
//             let mut rows = stmt.query(params![id]);
//             match rows {
//                 Ok(mut rows) => {
//                     match rows.next() {
//                         Ok(Some(row)) => {
//                             let row = row;
//                             let id: u64 = row.get(0)?;
//                             let user_id: u64 = row.get(1)?;
//                             let display_name: String = row.get(2)?;
//                             let points: u64 = row.get(3)?;
//                             let messages: u64 = row.get(4)?;
//                             Ok((id, user_id, display_name, points, messages))
//                         }
//                         Ok(None) => {
//                             log::debug!("user not found in db"); // trace because this is a normal occurrence and normally the next step is to add the user
//                             Err(Error::QueryReturnedNoRows)
//                         }
//                         Err(e) => {
//                             log::error!("error getting user from id: {}", e);
//                             Err(e)
//                         }
//                     }
//                 }
//                 Err(e) => {
//                     log::error!("error getting user from id: {}", e);
//                     Err(e)
//                 }
//             }
//         }
//         Err(e) => Err(e),
//     }
// }
//
// pub fn add_user(conn: &Connection, id: u64, tag: &String) -> Result<(), Error> {
//     log::debug!("executing: 'INSERT INTO discord_rank (user_id, display_name, points, messages) VALUES ({}, {}, {}, {})'", id, tag, 0, 0);
//     let q = conn.execute(
//         "INSERT INTO discord_rank (user_id, display_name, points, messages)
//         VALUES (?1, ?2, ?3, ?4)",
//         (id, tag, 0, 0),
//     );
//
//     match q {
//         Ok(_) => Ok(()),
//         Err(e) => {
//             log::error!("error adding user: {}", e);
//             Err(e)
//         }
//     };
//
//     Ok(())
// }
//
// pub fn check_if_user_exists(conn: &Connection, id: u64) -> Result<bool, Error> {
//     match get_user_from_id(conn, id) {
//         Ok(_) => Ok(true),
//         Err(e) => {
//             log::debug!("error getting user: {}", e);
//             Err(e)
//         }
//     }
// }
//
// pub fn update_name(conn: &Connection, id: u64, name: String) -> Result<(), Error> {
//     conn.execute(
//         "UPDATE discord_rank SET display_name = ?1 WHERE user_id = ?2",
//         params![name, id],
//     )?;
//     Ok(())
// }
//
// pub fn get_users(conn: &Connection) -> Result<Vec<(u64, String, u64, u64)>, Error> {
//     let mut stmt = conn.prepare("SELECT * FROM discord_rank")?;
//     let users = stmt.query_map([], |row| {
//         Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
//     })?;
//     let mut users_vec = Vec::new();
//     for user in users {
//         users_vec.push(user?);
//     }
//     Ok(users_vec)
// }
//
// pub fn update_user(conn: &Connection, id: u64, points: u64, messages: u64) -> Result<(), Error> {
//     conn.execute(
//         "UPDATE discord_rank SET points = ?1, messages = ?2 WHERE user_id = ?3",
//         [points, messages, id],
//     )?;
//     Ok(())
// }
//
// pub fn add_points(conn: &Connection, id: u64, points: u64) -> Result<(), Error> {
//     let user = get_user_from_id(&conn, id)?;
//     update_user(&conn, id, user.3 + points, user.4)?;
//     Ok(())
// }
//
// pub fn add_message(conn: &Connection, id: u64) -> Result<(), Error> {
//     let user = get_user_from_id(&conn, id)?;
//     update_user(&conn, id, user.3, user.4 + 1)?;
//     Ok(())
// }
