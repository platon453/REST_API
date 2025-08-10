#[macro_use]
extern crate rocket;

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::http::{Status};
use rocket::response::status;
use rocket::request::{Request, FromRequest};
use rusqlite::{params, Connection, Result, Error};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use chrono::{Utc, Duration};

// --- JWT & Auth Structures ---

const SECRET_KEY: &[u8] = b"your_super_secret_key";

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Claims {
    sub: String, // Subject (user email)
    exp: i64,    // Expiration time
}

// This struct will be the output of our request guard
struct AuthenticatedUser {
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct AuthResponse {
    token: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct NewUser {
    email: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct LoginCredentials {
    email: String,
    password: String,
}

#[derive(Debug)]
struct User {
    id: i32,
    email: String,
    password_hash: String,
}

// --- Post Structures ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Post {
    id: i32,
    title: String,
    body: String,
    author_email: String,
    created_at: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct NewPost {
    title: String,
    body: String,
}


// --- Database Functions ---

fn connect_db() -> Result<Connection> {
    let conn = Connection::open("database.db")?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    Ok(conn)
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            author_email TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (author_email) REFERENCES users (email)
        )",
        [],
    )?;
    Ok(())
}

// --- JWT Request Guard ---

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = String;

    async fn from_request(req: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let keys: Vec<_> = req.headers().get("Authorization").collect();
        if keys.len() != 1 {
            // Super explicit typing to try and fix the compiler error
            let res: rocket::request::Outcome<Self, Self::Error> = rocket::request::Outcome::Failure(Status::Unauthorized, "Invalid token header".to_string());
            return res;
        }
        let token_str = keys[0].replace("Bearer ", "");

        match decode::<Claims>(&token_str, &DecodingKey::from_secret(SECRET_KEY), &Validation::default()) {
            Ok(token_data) => rocket::request::Outcome::Success(AuthenticatedUser { email: token_data.claims.sub }),
            Err(_) => rocket::request::Outcome::Failure(Status::Unauthorized, "Invalid token".to_string()),
        }
    }
}


// --- Auth Endpoints ---

#[post("/auth/register", data = "<new_user>")]
async fn register(new_user: Json<NewUser>) -> status::Custom<String> {
    let conn = match connect_db() {
        Ok(c) => c,
        Err(_) => return status::Custom(Status::InternalServerError, "Database connection failed".to_string()),
    };
    if let Err(_) = create_tables(&conn) {
         return status::Custom(Status::InternalServerError, "Failed to create tables".to_string());
    }

    let password_hash = match hash(&new_user.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return status::Custom(Status::InternalServerError, "Failed to hash password".to_string()),
    };

    match conn.execute(
        "INSERT INTO users (email, password_hash) VALUES (?1, ?2)",
        params![new_user.email, password_hash],
    ) {
        Ok(_) => status::Custom(Status::Created, "User registered successfully".to_string()),
        Err(Error::SqliteFailure(_, Some(reason))) if reason.contains("UNIQUE constraint failed") => {
            status::Custom(Status::Conflict, "Email already exists".to_string())
        }
        Err(_) => status::Custom(Status::InternalServerError, "Failed to register user".to_string()),
    }
}

#[post("/auth/login", data = "<credentials>")]
async fn login(credentials: Json<LoginCredentials>) -> Result<Json<AuthResponse>, status::Custom<String>> {
    let conn = connect_db().map_err(|_| status::Custom(Status::InternalServerError, "Database connection failed".to_string()))?;

    let user = conn.query_row(
        "SELECT id, email, password_hash FROM users WHERE email = ?1",
        params![credentials.email],
        |row| {
            Ok(User {
                id: row.get(0)?,
                email: row.get(1)?,
                password_hash: row.get(2)?,
            })
        },
    );

    match user {
        Ok(u) => {
            if verify(&credentials.password, &u.password_hash).unwrap_or(false) {
                let expiration = Utc::now()
                    .checked_add_signed(Duration::hours(24))
                    .expect("Failed to create expiration")
                    .timestamp();

                let claims = Claims {
                    sub: u.email,
                    exp: expiration,
                };

                let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET_KEY))
                    .map_err(|_| status::Custom(Status::InternalServerError, "Failed to create token".to_string()))?;

                Ok(Json(AuthResponse { token }))
            } else {
                Err(status::Custom(Status::Unauthorized, "Invalid credentials".to_string()))
            }
        }
        Err(Error::QueryReturnedNoRows) => Err(status::Custom(Status::Unauthorized, "Invalid credentials".to_string())),
        Err(_) => Err(status::Custom(Status::InternalServerError, "Database query failed".to_string())),
    }
}

// --- Post Endpoints ---

#[post("/posts", data = "<new_post>")]
async fn create_post(new_post: Json<NewPost>, user: AuthenticatedUser) -> Result<Json<Post>, status::Custom<String>> {
    let conn = connect_db().map_err(|_| status::Custom(Status::InternalServerError, "Database connection failed".to_string()))?;

    let post_id = conn.execute(
        "INSERT INTO posts (title, body, author_email) VALUES (?1, ?2, ?3)",
        params![new_post.title, new_post.body, user.email],
    ).map_err(|_| status::Custom(Status::InternalServerError, "Failed to create post".to_string()))?;

    let created_post = conn.query_row(
        "SELECT id, title, body, author_email, created_at FROM posts WHERE id = ?1",
        params![post_id],
        |row| {
            Ok(Post {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                author_email: row.get(3)?,
                created_at: row.get(4)?,
            })
        },
    ).map_err(|_| status::Custom(Status::InternalServerError, "Failed to fetch created post".to_string()))?;

    Ok(Json(created_post))
}

#[get("/posts")]
async fn get_all_posts() -> Result<Json<Vec<Post>>, status::Custom<String>> {
    let conn = connect_db().map_err(|_| status::Custom(Status::InternalServerError, "Database connection failed".to_string()))?;
    let mut stmt = conn.prepare("SELECT id, title, body, author_email, created_at FROM posts ORDER BY created_at DESC")
        .map_err(|_| status::Custom(Status::InternalServerError, "Query preparation failed".to_string()))?;

    let post_iter = stmt.query_map([], |row| {
        Ok(Post {
            id: row.get(0)?,
            title: row.get(1)?,
            body: row.get(2)?,
            author_email: row.get(3)?,
            created_at: row.get(4)?,
        })
    }).map_err(|_| status::Custom(Status::InternalServerError, "Query mapping failed".to_string()))?;

    let posts = post_iter.filter_map(Result::ok).collect();
    Ok(Json(posts))
}

#[get("/posts/<id>")]
async fn get_post(id: i32) -> Result<Json<Post>, status::Custom<String>> {
    let conn = connect_db().map_err(|_| status::Custom(Status::InternalServerError, "Database connection failed".to_string()))?;
    let post = conn.query_row(
        "SELECT id, title, body, author_email, created_at FROM posts WHERE id = ?1",
        params![id],
        |row| {
            Ok(Post {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                author_email: row.get(3)?,
                created_at: row.get(4)?,
            })
        },
    );

    match post {
        Ok(p) => Ok(Json(p)),
        Err(Error::QueryReturnedNoRows) => Err(status::Custom(Status::NotFound, "Post not found".to_string())),
        Err(_) => Err(status::Custom(Status::InternalServerError, "Database query failed".to_string())),
    }
}

#[put("/posts/<id>", data = "<post_update>")]
async fn update_post(id: i32, post_update: Json<NewPost>, user: AuthenticatedUser) -> Result<Json<Post>, status::Custom<String>> {
    let conn = connect_db().map_err(|_| status::Custom(Status::InternalServerError, "Database connection failed".to_string()))?;

    // First, verify the author
    let post_author: Result<String, _> = conn.query_row(
        "SELECT author_email FROM posts WHERE id = ?1",
        params![id],
        |row| row.get(0),
    );

    match post_author {
        Ok(author_email) => {
            if author_email != user.email {
                return Err(status::Custom(Status::Forbidden, "You are not authorized to update this post".to_string()));
            }
        }
        Err(Error::QueryReturnedNoRows) => return Err(status::Custom(Status::NotFound, "Post not found".to_string())),
        Err(_) => return Err(status::Custom(Status::InternalServerError, "Database query failed".to_string())),
    }

    // If authorized, update the post
    conn.execute(
        "UPDATE posts SET title = ?1, body = ?2 WHERE id = ?3",
        params![post_update.title, post_update.body, id],
    ).map_err(|_| status::Custom(Status::InternalServerError, "Failed to update post".to_string()))?;

    get_post(id).await
}

#[delete("/posts/<id>")]
async fn delete_post(id: i32, user: AuthenticatedUser) -> status::Custom<String> {
    let conn = match connect_db() {
        Ok(c) => c,
        Err(_) => return status::Custom(Status::InternalServerError, "Database connection failed".to_string()),
    };

    // First, verify the author
    let post_author: Result<String, _> = conn.query_row(
        "SELECT author_email FROM posts WHERE id = ?1",
        params![id],
        |row| row.get(0),
    );

    match post_author {
        Ok(author_email) => {
            if author_email != user.email {
                return status::Custom(Status::Forbidden, "You are not authorized to delete this post".to_string());
            }
        }
        Err(Error::QueryReturnedNoRows) => return status::Custom(Status::NotFound, "Post not found".to_string()),
        Err(_) => return status::Custom(Status::InternalServerError, "Database query failed".to_string()),
    }

    // If authorized, delete the post
    match conn.execute("DELETE FROM posts WHERE id = ?1", params![id]) {
        Ok(0) => status::Custom(Status::NotFound, "Post not found".to_string()),
        Ok(_) => status::Custom(Status::Ok, "Post deleted successfully".to_string()),
        Err(_) => status::Custom(Status::InternalServerError, "Failed to delete post".to_string()),
    }
}


#[launch]
fn rocket() -> _ {
    rocket::build().mount(
        "/api",
        routes![
            register,
            login,
            create_post,
            get_all_posts,
            get_post,
            update_post,
            delete_post
        ],
    )
}