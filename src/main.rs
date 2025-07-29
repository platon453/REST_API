#[macro_use]
extern crate rocket;
use rocket::serde::{Deserialize, Serialize, json::Json};
use rusqlite::{Connection, Result, params};

// Структура для представления партнёра
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Partner {
    id: Option<i32>,
    name: String,
    full_name: String,
    phone: String,
    email: String,
    description: String,
    discount: f64, // Добавлено поле для скидки
}

// Структура для представления реализации
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Realise {
    id: Option<i32>,
    date: String,
    number: String,
    price: f64,
    customer_name: String,
}

// Соединение с базой данных SQLite
fn connect_db() -> Result<Connection> {
    let conn = Connection::open("database.db")?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    Ok(conn)
}

// Создание таблиц (если не существуют)
fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS partners (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            full_name TEXT NOT NULL,
            phone TEXT NOT NULL,
            email TEXT NOT NULL,
            description TEXT NOT NULL,
            discount REAL NOT NULL DEFAULT 0.0
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS realises (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            number TEXT NOT NULL,
            price REAL NOT NULL,
            customer_name TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

// CRUD для партнёров
#[post("/partners", data = "<partner>")]
async fn create_partner(partner: Json<Partner>) -> Json<Partner> {
    let conn = connect_db().expect("Unable to connect to DB");
    create_tables(&conn).expect("Unable to create tables");

    conn.execute(
        "INSERT INTO partners (name, full_name, phone, email, description, discount) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![partner.name, partner.full_name, partner.phone, partner.email, partner.description, partner.discount],
    ).expect("Failed to insert partner");

    Json(partner.into_inner())
}

#[get("/partners")]
async fn get_all_partners() -> Json<Vec<Partner>> {
    let conn = connect_db().expect("Unable to connect to DB");
    let mut stmt = conn
        .prepare("SELECT id, name, full_name, phone, email, description, discount FROM partners")
        .expect("Query failed");
    let partner_iter = stmt
        .query_map([], |row| {
            Ok(Partner {
                id: row.get(0)?,
                name: row.get(1)?,
                full_name: row.get(2)?,
                phone: row.get(3)?,
                email: row.get(4)?,
                description: row.get(5)?,
                discount: row.get(6)?,
            })
        })
        .expect("Mapping failed");

    Json(partner_iter.filter_map(Result::ok).collect())
}

#[put("/partners/<id>", data = "<partner>")]
async fn modify_partner(id: i32, partner: Json<Partner>) -> &'static str {
    let conn = connect_db().expect("Unable to connect to DB");
    let updated = conn
        .execute(
            "UPDATE partners SET
            name = ?1,
            full_name = ?2,
            phone = ?3,
            email = ?4,
            description = ?5,
            discount = ?6
            WHERE id = ?7",
            params![
                partner.name,
                partner.full_name,
                partner.phone,
                partner.email,
                partner.description,
                partner.discount,
                id
            ],
        )
        .expect("Error update partner");
    if updated > 0 {
        "Партнер обновлен"
    } else {
        "Партнер не найден"
    }
}

#[delete("/partners/<id>")]
async fn remove_partner(id: i32) -> &'static str {
    let conn = connect_db().expect("Unable to connect to DB");
    let deleted = conn
        .execute("DELETE FROM partners WHERE id = ?1", params![id])
        .expect("Failed to delete partner");
    if deleted > 0 {
        "Partner deleted successfully"
    } else {
        "Partners not found"
    }
}

#[post("/realises", data = "<realise>")]
async fn create_realise(realise: Json<Realise>) -> Json<Realise> {
    let conn = connect_db().expect("Unable to connect to DB");
    create_tables(&conn).expect("Unable to create tables");

    conn.execute(
        "INSERT INTO realises (data, number, price, customer_name) VALUES (?1, ?2, ?3,?4)",
        params![
            realise.date,
            realise.number,
            realise.price,
            realise.customer_name
        ],
    )
    .expect("Failed to insert realise");
    Json(realise.into_inner())
}

#[get("/realises")]
async fn get_all_realises() -> Json<Vec<Realise>> {
    let conn = connect_db().expect("Unable to connect to DB");
    let mut stmt = conn
        .prepare("SELECT id, date, number, price, customer_name FROM realises")
        .expect("Query failed");
    let realise_iter = stmt
        .query_map([], |row| {
            Ok(Realise {
                id: row.get(0)?,
                date: row.get(1)?,
                number: row.get(2)?,
                price: row.get(3)?,
                customer_name: row.get(4)?,
            })
        })
        .expect("Mapping failed");

    Json(realise_iter.filter_map(Result::ok).collect())
}

#[delete("/realises/<id>")]
async fn remove_realise(id: i32) -> &'static str {
    let conn = connect_db().expect("Unable to connect to DB");
    let deleted = conn
        .execute("DELETE FROM realises WHERE id = ?1", params![id])
        .expect("Failed to delete realise");

    if deleted > 0 {
        "Realise deleted successfully"
    } else {
        "Realise not found"
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount(
        "/",
        routes![
            create_partner,
            get_all_partners,
            modify_partner,
            remove_partner,
            create_realise,
            get_all_realises,
            remove_realise
        ],
    )
}
