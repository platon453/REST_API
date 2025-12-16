# REST API на Rust с SQLite

Этот проект представляет собой простой REST API, реализованный на языке Rust с использованием фреймворка [Rocket](https://rocket.rs/) и базы данных SQLite.

## Возможности API

- **Партнёры**
  - Создание партнёра (`POST /partners`)
  - Получение списка партнёров (`GET /partners`)
  - Изменение данных партнёра (`PUT /partners/<id>`)
  - Удаление партнёра (`DELETE /partners/<id>`)

- **Реализации**
  - Создание реализации (`POST /realises`)
  - Получение списка реализаций (`GET /realises`)
  - Удаление реализации (`DELETE /realises/<id>`)

## Установка и запуск

### Требования:
- Rust (установить можно через [rustup](https://rustup.rs/))
- Cargo (идёт вместе с Rust)
- SQLite (установить через пакетный менеджер)

### Запуск:

1. Клонировать репозиторий:
   ```sh
   git clone https://github.com/platon453/REST_API.git
   cd REST_API
   ```

2. Установить зависимости и собрать проект:
   ```sh
   cargo build
   ```

3. Запустить сервер:
   ```sh
   cargo run
   ```

4. API будет доступен по адресу `http://localhost:8000/`.

## Примеры использования (cURL)

### 1. Партнёры

#### Создание партнёра:
```sh
curl -X POST http://localhost:8000/partners \
     -H "Content-Type: application/json" \
     -d '{"name":"ООО Ромашка","full_name":"Общество с ограниченной ответственностью Ромашка","phone":"+79001112233","email":"info@romashka.ru","description":"Оптовый поставщик","discount":5.0}'
```

#### Получение списка партнёров:
```sh
curl -X GET http://localhost:8000/partners
```

#### Изменение данных партнёра:
```sh
curl -X PUT http://localhost:8000/partners/1 \
     -H "Content-Type: application/json" \
     -d '{"name":"ООО Лилия","full_name":"Общество с ограниченной ответственностью Лилия","phone":"+79001112234","email":"info@lilia.ru","description":"Поставщик бытовой техники","discount":10.0}'
```

#### Удаление партнёра:
```sh
curl -X DELETE http://localhost:8000/partners/1
```

### 2. Реализации

#### Создание реализации:
```sh
curl -X POST http://localhost:8000/realises \
     -H "Content-Type: application/json" \
     -d '{"date":"2025-03-14","number":"INV-1001","price":25000.50,"customer_name":"ООО Клиент"}'
```

#### Получение списка реализаций:
```sh
curl -X GET http://localhost:8000/realises
```

#### Удаление реализации:
```sh
curl -X DELETE http://localhost:8000/realises/1
```

## Лицензия
Проект распространяется под лицензией MIT.

