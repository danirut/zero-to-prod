use axum::Form;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String
}

pub async fn subscribe(Form(form): Form<FormData>) -> () {}