use leptos::*;

mod app;
mod components;
mod pages;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("Failed to initialize console_log");

    mount_to_body(|| {
        view! {
            <App />
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
