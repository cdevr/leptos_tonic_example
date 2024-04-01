use crate::error_template::{AppError, ErrorTemplate};
use futures::StreamExt;
use leptos::{
    server_fn::codec::{ByteStream, Streaming},
    *,
};
use leptos_meta::*;
use leptos_router::*;
use prost::Message;
use sha2::{Digest, Sha256};

#[component]
pub fn LoginWindow(is_logged_in: WriteSignal<bool>, username_handle: WriteSignal<String>) -> impl IntoView {
    let (username, set_username) = create_signal(String::new());

    #[server]
    pub async fn join(username: String) -> Result<bool, ServerFnError> {
        use backend::proto::chat_service_client::*;
        let mut client = ChatServiceClient::connect("https://[::1]:50051")
            .await
            .expect("Failed to esablish connection with backend");

        let request = tonic::Request::new(backend::proto::User{ id: "0".into(), name: username });

        let response = client.join(request).await.expect("failed to query login").into_inner();

        Ok(response.error == 0)
    }

    view! {
        <div class="card card-compact w-96 h-96 bg-base-100 shadow-xl">
            // <figure><img src="https://daisyui.com/images/stock/photo-1606107557195-0e29a4b5b4aa.jpg" alt="Shoes" /></figure>
            <div class="card-body">
                <h2 class="card-title justify-center">User Login</h2>
                <div>
                    <label class="input input-bordered flex items-center gap-2">
                      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="w-4 h-4 opacity-70"><path d="M8 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM12.735 14c.618 0 1.093-.561.872-1.139a6.002 6.002 0 0 0-11.215 0c-.22.578.254 1.139.872 1.139h9.47Z" /></svg>
                      <input type="text" class="grow" on:input=move |ev| {
                            set_username(event_target_value(&ev));
                        } prop:value=username placeholder="Username" />
                    </label>
                </div>
                <div class="card-actions justify-end">
                    <button class="btn btn-primary" on:click=move |_| {
                            spawn_local(async move {
                            if let Ok(true) = join(username.get()).await {
                                username_handle.set(username.get());
                                is_logged_in.set(true);
                            }
                        });
                    }>
                        Login
                    </button>
                </div>
            </div>
        </div>
    }
}

//This is the best way I could find to transfer the chat message from the server over to the
//client. Create a replicated struct of whats on server side using prost which has functinality to
//convert to and from bytes using prost::Message trait.
#[derive(prost::Message)]
struct ChatMessage {
    #[prost(string, tag = "1")]
    from: prost::alloc::string::String,
    #[prost(string, tag = "2")]
    msg: prost::alloc::string::String,
    #[prost(string, tag = "3")]
    time: prost::alloc::string::String,
}

fn sha256_username(username: &str) -> String {
    let mut hasher = Sha256::new();

    hasher.update(username.as_bytes());

    hex::encode(hasher
        .finalize()
        .to_vec())
}

#[component]
pub fn ChatWindow(username: String) -> impl IntoView {
    let (messages, set_messages) = create_signal(vec![]);

    #[cfg(feature = "ssr")]
    mod chat_recv {
        use backend::proto::*;
        use futures::Stream;

        pub async fn recv_message() -> impl Stream<Item = Result<ChatMessage, tonic::Status>> {
            use chat_service_client::ChatServiceClient;
            let mut client = ChatServiceClient::connect("http://[::1]:50051")
                .await
                .expect("Failed to establish connection with backend");

            let request = tonic::Request::new(Empty {});

            client
                .recieve_msg(request)
                .await
                .expect("Couldn't obtain stream")
                .into_inner()
        }
    }

    #[server(output = Streaming)]
    pub async fn handle_messages() -> Result<ByteStream, ServerFnError> {
        let stream = chat_recv::recv_message().await;

        // let data = stream.map(|message| Ok(message.unwrap().msg));
        let data = stream.map(|message| {
            let mut buf = vec![];
            message
                .unwrap()
                .encode(&mut buf)
                .expect("Couldn't convert to byte array");
            Ok(buf)
        });
        Ok(ByteStream::new(data))
    }

    create_effect(move |_| {
        spawn_local(async move {
            let mut stream = handle_messages()
                .await
                .expect("Couldn't initialize stream")
                .into_inner();

            while let Some(Ok(message)) = stream.next().await {
                set_messages.update(|messages| messages.push(message));
            }
        });
    });

    let chat_messages = move || {
        messages
            .get()
            .iter()
            .rev()
            .map(|message| Message::decode(&message[..]).expect("Failed to decode")) //https://www.gravatar.com/avatar/00000000000000000000000000000000?d=identicon&f=y
            .inspect(|msg: &ChatMessage| println!("{:?}", msg.from))
            .map(|message: ChatMessage| {
                view! {
                    <div class={ if message.from == username { "chat chat-start" } else { "chat chat-end" }}>
                        <div class="chat-image avatar">
                            <div class="w-10 rounded-full">
                                <img alt={format!("Gravatar Identicon for {}", message.from.clone())} 
                        src={ 
                            format!("https://www.gravatar.com/avatar/{}?d=identicon&f=y", sha256_username(&message.from))}
                        />
                            </div>
                        </div>
                        <div class="chat-header flex gap-2">
                            {message.from}
                            <time class="text xs opacity-50">{message.time}</time>
                        </div>
                        <div class="chat chat-bubble">{message.msg}</div>
                    </div>
                }
            })
            .collect_view()
    };

    view! {
        <div class="overflow-auto flex flex-col-reverse flex-[0_0_90vh] h-full">{chat_messages}</div>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/frontend.css"/>
        <link href="https://cdn.jsdelivr.net/npm/daisyui@4.7.3/dist/full.min.css" rel="stylesheet" type="text/css" />
        <script src="https://cdn.tailwindcss.com"></script>


        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {
                <ErrorTemplate outside_errors/>
            }
            .into_view()
        }>
            <main class="min-h-screen flex flex-col">
                <Routes>
                    <Route path="" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let (username, set_username) = create_signal(String::new());
    let (message, set_message) = create_signal(String::new());
    let (logged_in, set_logged_in) = create_signal(false);

    view! {
        { move || if !logged_in() {
                view!{
                    <div class="flex place-items-center justify-center w-full min-h-screen">
                    <LoginWindow is_logged_in=set_logged_in username_handle=set_username/>
                    </div>
                }.into_view()
            } else {
                view!{
                    <ChatWindow username=username.get()/>
                    <div class="flex flex-1 place-content-center gap-1">
                        <input type="text" class="input input-bordered flex-[0_0_80vw]" on:input=move |ev| {
                            set_message(event_target_value(&ev));
                        } prop:value=message placeholder="Enter text here."/>
                        <button class="btn btn-primary" on:click=move |_| {
                            let message = message.get();
                            let username = username.get();

                            spawn_local(async { let _ = send_message(username, message).await; });
                            set_message("".into());
                        }>"Send"</button>
                    </div>
                }.into_view()
            }
        }
    }
}

#[server]
pub async fn send_message(from: String, msg: String) -> Result<(), ServerFnError> {
    use chrono::{Local, Timelike};
    use backend::proto::chat_service_client::ChatServiceClient;
    let mut client = ChatServiceClient::connect("http://[::1]:50051").await?;
    let current_time = Local::now();

    let request = tonic::Request::new(backend::proto::ChatMessage {
        from,
        msg,
        time: format!("{}:{}", current_time.hour(), current_time.minute()),
    });


    let response = client.send_msg(request).await?;

    Ok(())
}
