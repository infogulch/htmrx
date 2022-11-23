#![allow(incomplete_features)]
#![feature(adt_const_params)]

#[macro_use]
extern crate rocket;

mod header;

use std::sync::{Mutex, RwLock};

use header::*;
use html_to_string_macro::html;
use rocket::{
    form::{Form, FromFormField},
    response::content::RawHtml,
};

fn page(title: String, body: String) -> String {
    html! {
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
    }
}

fn nav(title: String, content: String) -> String {
    html! {
        <head><title>{ title }</title></head>
        { content }
    }
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
        <form action="/?increment" method="POST" hx-boost="true" hx-target="#about-count" hx-swap="outerHTML">
            <button>"Increment"</button>
        </form>
        <p>"The counter is currently at: " { f_about_count() }</p>
    }
}

fn f_about_count() -> String {
    html! { <span id="about-count">{ *COUNTER.lock().unwrap() }</span> }
}

struct Item {
    id: u32,
    done: bool,
    label: String,
}

#[derive(strum::EnumString, PartialEq, Clone, Copy, FromFormField)]
enum Filter {
    All,
    Active,
    Completed,
}

static TODO_ITEMS: RwLock<Vec<Item>> = RwLock::new(vec![]);
static TODO_INC: Mutex<u32> = Mutex::new(0);
static TODO_FILTER: Mutex<Filter> = Mutex::new(Filter::All);

fn f_todos() -> String {
    html! {
        { f_tabs(Tabs::Todos) }
        <section class="todos">
            <header>
                <h1>"todos"</h1>
                <form action="/todos" method="POST" hx-boost="true" hx-target="none">
                { f_todos_toggleall(false) }
                { f_todos_input(false) }
                </form>
            </header>
            { f_todos_items(false) }
            <footer>
                { f_todos_count(false) }
                { f_todos_filter() }
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

fn f_todos_items(oob: bool) -> String {
    use Filter::*;
    let filter = *TODO_FILTER.lock().unwrap();
    let filter_fn = move |i: &'_ &Item| match filter {
        All => true,
        Active => !i.done,
        Completed => i.done,
    };
    html! {
        <form action="/todos;apply" method="POST" hx-boost="true" hx-swap="none">
        <ul id="items-list" { if oob { r#"hx-swap-oob="true""# } else { "" } }>
            { TODO_ITEMS.read().unwrap().iter().rev().filter(filter_fn).map(f_todos_item).collect::<String>() }
        </ul>
        <noscript><button>"Apply"</button></noscript>
        </form>
    }
}

fn f_todos_item(item: &Item) -> String {
    let id = item.id;
    html! {
        <li hx-target="this" hx-swap="outerHTML">
            <input type="checkbox" { if item.done { "checked" } else { "" }} hx-post={ format!("/todos/{id}?toggle") }/>
            <label>{ item.label.as_str() }</label>
            <button class="delete" hx-delete={ format!("/todos/{id}") }></button>
        </li>
    }
}

fn f_todos_toggleall(oob: bool) -> String {
    let alldone = {
        let items = TODO_ITEMS.read().unwrap();
        items.len() > 0 && items.iter().all(|i| i.done)
    };

    html! {
        <input id="toggle-all" type="checkbox" { if alldone { "checked" } else { "" } }
            hx-post="/todos?toggleall" hx-target="this" { if oob { r#"hx-swap-oob="true""# } else { "" } } />
    }
}

fn f_todos_input(oob: bool) -> String {
    html! {
        <input id="todo-new" name="todo-new" placeholder="What needs to be done?" autofocus
        hx-post="/todos" hx-target=".todos ul" hx-swap="afterbegin" { if oob { r#"hx-swap-oob="true""# } else { "" } } />
    }
}

fn f_todos_count(oob: bool) -> String {
    let len = TODO_ITEMS
        .read()
        .unwrap()
        .iter()
        .filter(|i| !i.done)
        .count();
    html! {
        <span id="todo-count" { if oob { r#"hx-swap-oob="true""# } else { "" } }><strong>{ len }</strong>" item" { if len == 1 { "" } else { "s" } } " left"</span>
    }
}

fn f_todos_filter() -> String {
    use Filter::*;
    let selected_filter = *(TODO_FILTER.lock().unwrap());
    html! {
    <fieldset class="filter" hx-swap="none">
        <legend>"Filter"</legend>
        <input type="radio" id="filter-all" name="filter" value="All" { if selected_filter == All {"checked"} else {""} }  hx-get="./todos?filter=all" />
        <label for="filter-all">"All"</label>
        <input type="radio" id="filter-active" name="filter" value="Active" { if selected_filter == Active {"checked"} else {""} } hx-get="./todos?filter=active" />
        <label for="filter-active">"Active"</label>
        <input type="radio" id="filter-completed" name="filter" value="Completed" { if selected_filter == Completed {"checked"} else {""} } hx-get="./todos?filter=completed" />
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

#[get("/")]
fn about(hx: Option<HXRequest>) -> RawHtml<String> {
    let renderer = match hx {
        None => page,
        Some(_) => nav,
    };
    RawHtml(renderer("About".into(), f_about()))
}

#[post("/?increment")]
fn about_increment(hx: Option<HXRequest>) -> RawHtml<String> {
    {
        *COUNTER.lock().unwrap() += 1;
    }
    match hx {
        None => about(None),
        Some(_) => RawHtml(f_about_count()),
    }
}

#[get("/todos")]
fn todos(hx: Option<HXRequest>) -> RawHtml<String> {
    let handler = match hx {
        None => page,
        Some(_) => nav,
    };
    RawHtml(handler("Todos".into(), f_todos()))
}

#[derive(FromForm)]
struct NewTodo<'r> {
    #[field(name = "todo-new")]
    todo_new: &'r str,
}

#[post("/todos", data = "<item>")]
fn todos_add(item: Form<NewTodo>, hx: Option<HXRequest>) -> RawHtml<String> {
    let id = {
        let mut inc = TODO_INC.lock().unwrap();
        *inc = inc.wrapping_add(1);
        *inc
    };
    TODO_ITEMS.write().unwrap().push(Item {
        id: id,
        done: false,
        label: item.todo_new.to_string(),
    });
    match hx {
        None => todos(None),
        Some(_) => RawHtml(html! {
            { f_todos_item(TODO_ITEMS.read().unwrap().last().unwrap()) }
            { f_todos_input(true) }
            { f_todos_count(true) }
            { f_todos_toggleall(true) }
        }),
    }
}

#[delete("/todos/<id>")]
fn todos_del(id: u32, hx: Option<HXRequest>) -> RawHtml<String> {
    {
        let mut items = TODO_ITEMS.write().unwrap();
        if let Some(index) = items.iter().position(|i| i.id == id) {
            items.remove(index);
        } else {
            return RawHtml("Error".to_string());
        }
    }
    match hx {
        None => todos(None),
        Some(_) => RawHtml(html! {
            { f_todos_count(true) }
            { f_todos_toggleall(true) }
        }),
    }
}

#[post("/todos/<id>?toggle")]
fn todos_toggle(id: u32, hx: Option<HXRequest>) -> RawHtml<String> {
    let idx = {
        match TODO_ITEMS
            .write()
            .unwrap()
            .iter_mut()
            .enumerate()
            .filter(|(_, i)| i.id == id)
            .next()
        {
            Some((idx, item)) => {
                item.done = !item.done;
                idx
            }
            _ => return RawHtml(html! { <p>"Invalid item number"</p> }),
        }
    };
    match hx {
        None => todos(None),
        Some(_) => {
            let filter = *(TODO_FILTER.lock().unwrap());
            RawHtml(html! {
                { if filter == Filter::All {
                    f_todos_item(&TODO_ITEMS.read().unwrap()[idx])
                } else { "".to_string() } }
                { f_todos_count(true) }
                { f_todos_toggleall(true) }
            })
        }
    }
}

#[post("/todos?toggleall")]
fn todos_toggleall() -> RawHtml<String> {
    let set = !TODO_ITEMS.read().unwrap().iter().all(|i| i.done);
    let mut dirty = false;
    for mut item in TODO_ITEMS.write().unwrap().iter_mut() {
        dirty = dirty || item.done != set;
        item.done = set;
    }
    RawHtml(html! {
        { f_todos_count(true) }
        { f_todos_toggleall(true) }
        { if dirty { f_todos_items(true) } else { "".to_string() } }
    })
}

#[get("/todos?<filter>")]
fn todos_filter(filter: Filter, hx: Option<HXRequest>) -> RawHtml<String> {
    *(TODO_FILTER.lock().unwrap()) = filter;
    match hx {
        None => todos(None),
        Some(_) => RawHtml(f_todos_items(true)),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .configure(&rocket::Config {
            address: std::net::Ipv4Addr::new(0, 0, 0, 0).into(),
            ..rocket::Config::debug_default()
        })
        .mount(
            "/",
            routes![
                about,
                about_increment,
                todos,
                todos_add,
                todos_del,
                todos_toggle,
                todos_toggleall,
                todos_filter,
            ],
        )
}
