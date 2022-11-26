#![feature(never_type)]

use std::sync::{Arc, Mutex, RwLock};

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, Query, State},
    http::{
        header::{self, HeaderName},
        request::Parts,
        StatusCode,
    },
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Form, RequestPartsExt, Router, TypedHeader,
};
use headers::{Header, HeaderValue};
use html_to_string_macro::html;
use serde::{Deserialize, Serialize};
use tokio::main;

#[derive(Debug)]
struct HXRequest(bool);

impl Header for HXRequest {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("hx-request");
        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;

        if value == "false" {
            Ok(HXRequest(false))
        } else if value == "true" {
            Ok(HXRequest(true))
        } else {
            Err(headers::Error::invalid())
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let s = if self.0 { "true" } else { "false" };

        let value = HeaderValue::from_static(s);

        values.extend(std::iter::once(value));
    }
}

#[derive(PartialEq, Eq, Debug)]
struct IsHXRequest(bool);

#[async_trait]
impl<S> FromRequestParts<S> for IsHXRequest
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let hx: Option<TypedHeader<HXRequest>> = parts.extract().await.unwrap();
        Ok(IsHXRequest(match hx {
            Some(TypedHeader(HXRequest(true))) => true,
            _ => false,
        }))
    }
}

fn page(title: String, body: String) -> Response {
    Html(html! {
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8" />
            <title>{ title }</title>
            <link rel="stylesheet" href="https://unpkg.com/water.css@2.1.1/out/water.css"
              integrity="sha384-eHoWBq4xGyEfS3rmZe6gvzlNS/nNJhiPPbKCJN1cQHJukU+q6ji3My2fJGYd1EBo"
              crossorigin="anonymous" />
            <script src="https://unpkg.com/htmx.org@1.8.2/dist/htmx.js"
              integrity="sha384-dUlt2hvoUDyqJ29JH9ln6o/B23lVQiQm8Z0+oEuPBWwKXiyG2MozxxFsCKWM7dLl"
              crossorigin="anonymous"></script>
        </head>
        <body>
            { body }
        </body>
        </html>
    })
    .into_response()
}

fn nav(title: String, content: String) -> Response {
    Html(html! {
        <head><title>{ title }</title></head>
        { content }
    })
    .into_response()
}

fn frag(content: String) -> Response {
    Html(content).into_response()
}

#[derive(PartialEq, Eq)]
enum Tabs {
    About,
    Todos,
}

fn f_tabs(selected: Tabs) -> String {
    use Tabs::*;
    let (sel_about, sel_todos) = match (selected, r#"class="selected""#, "") {
        (About, sel, no) => (sel, no),
        (Todos, sel, no) => (no, sel),
    };
    html! {
    <div class="nav-tabs" hx-boost="true">
        <a href="/" { sel_about }>"About"</a>
        <a href="/todos" { sel_todos }>"Todos"</a>
        <style>r#"
            .nav-tabs>a {
                padding: 1em;
                color: var(--text-bright);
            }
            .nav-tabs>a:hover {
                background-color: var(--button-hover) !important;
            }
            .nav-tabs>a.selected {
                background-color: var(--button-base);
            }
            "#
        </style>
    </div>
    }
}

static COUNTER: Mutex<u32> = Mutex::new(0);

fn f_about() -> String {
    html! {
        { f_tabs(Tabs::About) }
        <br />
        <p>"This site demonstrates HTMX + Rust"</p>
        <form action="./" method="POST" hx-boost="true" hx-target="#about-count" hx-swap="outerHTML">
            <button>"Increment"</button>
        </form>
        <p>"The counter is currently at: " { f_about_count() }</p>
    }
}

fn f_about_count() -> String {
    html! { <span id="about-count">{ *COUNTER.lock().unwrap() }</span> }
}

async fn about_index(hx: IsHXRequest) -> impl IntoResponse {
    let renderer = if !hx.0 { page } else { nav };
    renderer("About".into(), f_about())
}

async fn about_increment(hx: IsHXRequest) -> impl IntoResponse {
    {
        *COUNTER.lock().unwrap() += 1;
    }
    if !hx.0 {
        return page("About".into(), f_about());
    }
    frag(f_about_count())
}

#[derive(Debug, Serialize, Clone)]
struct Todo {
    id: u32,
    completed: bool,
    text: String,
}

#[derive(strum::EnumString, Deserialize, PartialEq, Clone, Copy, Default)]
enum Filter {
    #[default]
    All,
    Active,
    Completed,
}

fn f_todos(db: &DbData) -> String {
    html! {
        { f_tabs(Tabs::Todos) }
        <section class="todos">
            <header>
                <h1>"todos"</h1>
                <form action="/todos" method="POST" hx-boost="true" hx-target="none">
                { f_todos_toggleall(db, false) }
                { f_todos_input(false) }
                </form>
            </header>
            { f_todos_items(db, false) }
            <footer>
                { f_todos_count(db, false) }
                { f_todos_filter(db) }
            </footer>
            <style>r#"
            .todos input {
                display: inline-block;
                vertical-align: middle;
            }
            header input[type=checkbox] {
                font-size: 1.5em;
            }
            .todos ul {
                list-style-type: none;
                padding-left: 0;
            }
            .todos li {
                position: relative;
                height: 2em;
                width: 20em;
                overflow: hidden;
                padding-top: .2em;
            }
            .todos li>* {
                vertical-align: middle;
            }
            .todos li>input[type=checkbox]:checked + label {
                text-decoration: line-through;
                color: var(--text-muted);
            }
            .todos button.delete:after {
                content: '×';
                font-size: 2em;
                position: relative;
                top: -.2em;
            }
            .todos button.delete {
                position: absolute;
                margin-top: -.2em;
                right: 0;
                padding: 0;
                color: #af5b5e;
                background-color: var(--background-body);
            }
            .todos button.delete:hover {
                color: #ac262b;
            }

            "#
            </style>
        </section>
    }
}

fn f_todos_items(db: &DbData, oob: bool) -> String {
    use Filter::*;
    let filter = db.filter;
    let filter_fn = move |i: &'_ &Todo| match filter {
        All => true,
        Active => !i.completed,
        Completed => i.completed,
    };
    html! {
        <form action="/todos;apply" method="POST" hx-boost="true" hx-swap="none">
        <ul id="items-list" { if oob { r#"hx-swap-oob="true""# } else { "" } }>
            { db.todos.iter().rev().filter(filter_fn).map(f_todos_item).collect::<String>() }
        </ul>
        <noscript><button>"Apply"</button></noscript>
        </form>
    }
}

fn f_todos_item(item: &Todo) -> String {
    let id = item.id;
    html! {
        <li hx-target="this" hx-swap="outerHTML">
            <input type="checkbox" { if item.completed { "checked" } else { "" }} hx-post={ format!("/todos/todo/{id}/toggle") }/>
            <label>{ item.text.as_str() }</label>
            <button class="delete" hx-delete={ format!("/todos/todo/{id}") }></button>
        </li>
    }
}

fn f_todos_toggleall(db: &DbData, oob: bool) -> String {
    let alldone = { db.todos.len() > 0 && db.todos.iter().all(|i| i.completed) };

    html! {
        <input id="toggle-all" type="checkbox" { if alldone { "checked" } else { "" } }
            hx-post="/todos/toggleall" hx-target="this" { if oob { r#"hx-swap-oob="true""# } else { "" } } />
    }
}

fn f_todos_input(oob: bool) -> String {
    html! {
        <input id="todo-new" name="text" placeholder="What needs to be done?" autofocus
        hx-post="/todos" hx-target=".todos ul" hx-swap="afterbegin" { if oob { r#"hx-swap-oob="true""# } else { "" } } />
    }
}

fn f_todos_count(db: &DbData, oob: bool) -> String {
    let len = db.todos.iter().filter(|i| !i.completed).count();
    html! {
        <span id="todo-count" { if oob { r#"hx-swap-oob="true""# } else { "" } }><strong>{ len }</strong>" item" { if len == 1 { "" } else { "s" } } " left"</span>
    }
}

fn f_todos_filter(db: &DbData) -> String {
    use Filter::*;
    let selected_filter = db.filter;
    html! {
    <fieldset class="filter" hx-swap="none">
        <legend>"Filter"</legend>
        <input type="radio" id="filter-all" name="mode" value="All" { if selected_filter == All {"checked"} else {""} } hx-get="/todos/filter" />
        <label for="filter-all">"All"</label>
        <input type="radio" id="filter-active" name="mode" value="Active" { if selected_filter == Active {"checked"} else {""} } hx-get="/todos/filter" />
        <label for="filter-active">"Active"</label>
        <input type="radio" id="filter-completed" name="mode" value="Completed" { if selected_filter == Completed {"checked"} else {""} } hx-get="/todos/filter" />
        <label for="filter-completed">"Completed"</label>
    </fieldset>
    <style>r#"
    .filter {
        max-width: fit-content;
    }
    .filter input[type=radio] {
        display: none;
    }
    .filter label {
        margin-left: .5em;
        padding: .3em;
        min-width: 3em;
        text-align: center;
        border: .1em solid var(--text-muted);
        border-radius: .5em;
    }
    .filter label:hover {
        border-color: var(--highlight);
        color: var(--highlight);
        box-shadow: 0px 0px .1em .1em var(--highlight);
    }
    .filter input:checked + label {
        border-color: var(--text-bright);
        color: var(--text-bright);
        background-color: var(--button-base);
    }
    "#
    </style>
    }
}

async fn todos_index(hx: IsHXRequest, State(state): State<Db>) -> impl IntoResponse {
    let handler = if !hx.0 { page } else { nav };
    handler("Todos".into(), f_todos(&state.read().unwrap()))
}

#[derive(Deserialize)]
struct CreateTodo {
    text: String,
}

async fn todos_create(
    hx: IsHXRequest,
    State(state): State<Db>,
    Form(todo): Form<CreateTodo>,
) -> impl IntoResponse {
    state.write().unwrap().create(todo.text);
    if !hx.0 {
        return todos_index(hx, State(state)).await.into_response();
    }
    let db = state.read().unwrap();
    frag(html! {
        { f_todos_item(db.todos.last().unwrap()) }
        { f_todos_input(true) }
        { f_todos_count(&db, true) }
        { f_todos_toggleall(&db, true) }
    })
}

async fn todos_toggleall(hx: IsHXRequest, State(state): State<Db>) -> impl IntoResponse {
    state.write().unwrap().toggleall();
    if !hx.0 {
        return todos_index(hx, State(state)).await.into_response();
    }
    let db = state.read().unwrap();
    frag(html! {
        { f_todos_count(&db, true) }
        { f_todos_toggleall(&db, true) }
        { f_todos_items(&db, true) }
    })
}

#[derive(Deserialize)]
struct TodosFilter {
    mode: Filter,
}

async fn todos_filter(
    hx: IsHXRequest,
    State(state): State<Db>,
    Query(TodosFilter { mode }): Query<TodosFilter>,
) -> impl IntoResponse {
    state.write().unwrap().filter = mode;
    if !hx.0 {
        return todos_index(hx, State(state)).await.into_response();
    }
    let db = state.read().unwrap();
    frag(f_todos_items(&db, true))
}

async fn todos_toggle(
    hx: IsHXRequest,
    Path(id): Path<u32>,
    State(state): State<Db>,
) -> impl IntoResponse {
    let idx = match state.write().unwrap().toggle(id) {
        Ok(x) => x,
        _ => return (StatusCode::BAD_REQUEST).into_response(),
    };
    if !hx.0 {
        return todos_index(hx, State(state)).await.into_response();
    }
    let db = state.read().unwrap();
    frag(html! {
        { if db.filter == Filter::All {
            f_todos_item(&db.todos[idx])
        } else { "".to_string() } }
        { f_todos_count(&db, true) }
        { f_todos_toggleall(&db, true) }
    })
}

async fn todos_delete(
    hx: IsHXRequest,
    Path(id): Path<u32>,
    State(state): State<Db>,
) -> impl IntoResponse {
    if let Err(()) = state.write().unwrap().delete(id) {
        return (StatusCode::BAD_REQUEST).into_response();
    }
    if !hx.0 {
        return todos_index(hx, State(state)).await.into_response();
    }
    let db = state.read().unwrap();
    frag(html! {
        { f_todos_count(&db, true) }
        { f_todos_toggleall(&db, true) }
    })
}

#[main]
async fn main() {
    const TODOS_PATH: &'static str = "/todos";

    let db: Db = Arc::new(RwLock::new(DbData {
        mount_path: TODOS_PATH,
        ..Default::default()
    }));

    let todos_app = Router::new()
        .route("/", get(todos_index).post(todos_create))
        .route("/filter", get(todos_filter))
        .route("/toggleall", post(todos_toggleall))
        .route("/todo/:id", delete(todos_delete))
        .route("/todo/:id/toggle", post(todos_toggle))
        .with_state(db);

    // Compose the routes
    let app = Router::new()
        .nest(TODOS_PATH, todos_app)
        .route("/", get(about_index).post(about_increment));

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Default)]
struct DbData {
    mount_path: &'static str,
    inc: u32,
    filter: Filter,
    todos: Vec<Todo>,
}

impl DbData {
    fn create(&mut self, text: String) {
        self.inc += 1;
        self.todos.push(Todo {
            id: self.inc,
            completed: false,
            text: text,
        });
    }

    fn delete(&mut self, id: u32) -> Result<(), ()> {
        if let Some(index) = self.todos.iter().position(|i| i.id == id) {
            self.todos.remove(index);
            Ok(())
        } else {
            Err(())
        }
    }

    fn toggle(&mut self, id: u32) -> Result<usize, ()> {
        match self
            .todos
            .iter_mut()
            .enumerate()
            .filter(|(_, i)| i.id == id)
            .next()
        {
            Some((idx, item)) => {
                item.completed = !item.completed;
                Ok(idx)
            }
            None => Err(()),
        }
    }

    fn toggleall(&mut self) {
        let set = !self.todos.iter().all(|i| i.completed);
        for mut item in self.todos.iter_mut() {
            item.completed = set;
        }
    }
}

type Db = Arc<RwLock<DbData>>;
