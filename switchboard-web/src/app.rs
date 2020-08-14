use wasm_bindgen::prelude::*;
use yew::prelude::*;

use log::*;
use wasm_bindgen_futures::spawn_local;

use crate::rtc;

pub struct App {
    link: ComponentLink<Self>,
    localVideo: NodeRef,
    value: i64,
}

pub enum Msg {
    StartLocalStream,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            value: 0,
            localVideo: NodeRef::default(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::StartLocalStream => spawn_local(async {
                let stream = rtc::init_user_media().await.unwrap();
                debug!("got stream: ");
                //                self.localVideo.cast::<Element>();
            }),
        }
        true
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        // Should only return "true" if new properties are different to
        // previously received properties.
        // This component has no properties so we will always return "false".
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <button onclick=self.link.callback(|_| Msg::StartLocalStream)>{ "Start Local Stream" }</button>
                <video ref=self.localVideo.clone()/>
                <p>{ self.value }</p>
            </div>
        }
    }
}
