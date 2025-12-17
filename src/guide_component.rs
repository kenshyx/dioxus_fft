use dioxus::prelude::*;

#[component]
pub fn Chapter3() -> Element {
    for number in (1..3).rev() {
        tracing::info!("got {number}")
    }
    rsx! {
            div { id: "title",
                h1 { "HotDog! 🌭" }
            }
            div { id: "dogview",
                img { src: "https://images.dog.ceo/breeds/pitbull/dog-3981540_1280.jpg", alt: "doggy" }
            }
            div { id: "buttons",
                button { id: "skip", "skip" }
                button { id: "save", "save!" }
        }
    }
}
