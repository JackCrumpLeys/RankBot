use rusqlite::{Connection, Error, params};
use serenity::model::prelude::{Message, User};
use serenity::model::Timestamp;

pub fn get_connection() -> Result<Connection, Error> {
    Ok(Connection::open("db.sqlite")?)
}

pub fn create_db(conn: Connection) -> Result<(), Error> {
    // database for discord rank bot data. stuff like points and messages.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS discord_rank (
            id INTEGER PRIMARY KEY,
            user_id UNSIGNED BIG INT NOT NULL,
            display_name TEXT NOT NULL,
            points INTEGER UNSIGNED NOT NULL,
            messages INTEGER UNSIGNED NOT NULL
        )",
        [],)?;
    // store messages for analysis
    conn.execute(
        "CREATE TABLE IF NOT EXISTS discord_messages (
            id INTEGER PRIMARY KEY,
            user INTEGER UNSIGNED NOT NULL,
            message TEXT NOT NULL,
            timestamp DATETIME NOT NULL
        )",
        [],)?;
    Ok(())
}

// ----------------------------------------------------------------
// messages
// ----------------------------------------------------------------

pub fn store_message(conn: &Connection, user_id: u64, content: String, timestamp:i64) -> Result<(), Error> {
    let user_db_id = get_user_from_id(&conn, user_id)?.0;
    log::debug!("executing: 'INSERT INTO discord_messages (user, message, timestamp) VALUES ({}, {}, {})'", user_db_id, content, timestamp);


    let q = conn.execute(
        "INSERT INTO discord_messages (user, message, timestamp)
        VALUES (?1, ?2, ?3)",
        params![user_db_id, content, timestamp]);

    match q {
        Ok(_) => Ok(()),
        Err(e) => Err(e)
    };

    Ok(())
}

pub fn get_messages_from_user(conn: &Connection, id: u64) -> Result<Vec<String>, Error> {
    let mut stmt = conn.prepare("SELECT message FROM discord_messages WHERE user = ?1")?;
    let messages = stmt.query_map([id], |row| row.get(0))?;
    let mut messages_vec = Vec::new();
    for message in messages {
        messages_vec.push(message?);
    }
    Ok(messages_vec)
}

pub fn get_messages(conn: &Connection) -> Result<Vec<String>, Error> {
    let mut stmt = conn.prepare("SELECT message FROM discord_messages")?;
    let messages = stmt.query_map([], |row| row.get(0))?;
    let mut messages_vec = Vec::new();
    for message in messages {
        messages_vec.push(message?);
    }
    Ok(messages_vec)
}

// ----------------------------------------------------------------
// users
// ----------------------------------------------------------------

pub fn get_user_from_id(conn: &Connection, id: u64) -> Result<(u64, u64, String, u64, u64), Error> {
    log::debug!("executing: 'SELECT * FROM discord_rank WHERE user_id = {}'", id);
    let mut stmt = conn.prepare("SELECT * FROM discord_rank WHERE user_id = ?1");
    match stmt {
        Ok(mut stmt) => {
            let mut rows = stmt.query(params![id]);
            match rows {
                Ok(mut rows) => {
                    match rows.next() {
                        Ok(Some(row)) => {
                            let row = row;
                            let id: u64 = row.get(0)?;
                            let user_id: u64 = row.get(1)?;
                            let display_name: String = row.get(2)?;
                            let points: u64 = row.get(3)?;
                            let messages: u64 = row.get(4)?;
                            Ok((id, user_id, display_name, points, messages))
                        },
                        Ok(None) => {
                            log::debug!("user not found in db"); // trace because this is a normal occurrence and normally the next step is to add the user
                            Err(Error::QueryReturnedNoRows)
                        }
                        Err(e) => {
                            log::error!("error getting user from id: {}", e);
                            Err(e)
                        }
                    }
                },
                Err(e) => {
                    log::error!("error getting user from id: {}", e);
                    Err(e)
                }
            }

        },
        Err(e) => Err(e)
    }
}

pub fn add_user(conn: &Connection, id: u64, tag: &String) -> Result<(), Error> {
    log::debug!("executing: 'INSERT INTO discord_rank (user_id, display_name, points, messages) VALUES ({}, {}, {}, {})'", id, tag, 0, 0);
    let q = conn.execute(
        "INSERT INTO discord_rank (user_id, display_name, points, messages)
        VALUES (?1, ?2, ?3, ?4)",
        (id, tag, 0, 0),
    );

    match q {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("error adding user: {}", e);
            Err(e)
        }
    };

    Ok(())
}

pub fn check_if_user_exists(conn: &Connection, id: u64) -> Result<bool, Error> {
    match get_user_from_id(conn, id) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::debug!("error getting user: {}", e);
            Err(e)
        }
    }
}

pub fn update_name(conn: &Connection, id: u64, name: String) -> Result<(), Error> {
    conn.execute(
        "UPDATE discord_rank SET display_name = ?1 WHERE user_id = ?2",
        params![name, id],
    )?;
    Ok(())
}

pub fn get_users(conn: &Connection) -> Result<Vec<(u64, String, u64, u64)>, Error> {
    let mut stmt = conn.prepare("SELECT * FROM discord_rank")?;
    let users = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?;
    let mut users_vec = Vec::new();
    for user in users {
        users_vec.push(user?);
    }
    Ok(users_vec)
}

pub fn update_user(conn: &Connection, id: u64, points: u64, messages: u64) -> Result<(), Error> {
    conn.execute(
        "UPDATE discord_rank SET points = ?1, messages = ?2 WHERE user_id = ?3",
        [points, messages, id],
    )?;
    Ok(())
}

pub fn add_points(conn: &Connection, id: u64, points: u64) -> Result<(), Error> {
    let user = get_user_from_id(&conn, id)?;
    update_user(&conn, id, user.3 + points, user.4)?;
    Ok(())
}

pub fn add_message(conn: &Connection, id: u64) -> Result<(), Error> {
    let user = get_user_from_id(&conn, id)?;
    update_user(&conn, id, user.3, user.4 + 1)?;
    Ok(())
}
