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
pub fn ChatWindow() -> impl IntoView {
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
            .map(|message: ChatMessage| {
                view! {
                    <div class="chat chat-start">
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
    let (message, set_message) = create_signal(String::from("Enter text here."));

    view! {
        {ChatWindow()}
        <div class="flex flex-1 place-content-center gap-1">
            <input type="text" class="input input-bordered flex-[0_0_80vw]" on:input=move |ev| {
                set_message(event_target_value(&ev));
            } prop:value=message/>
            <button class="btn btn-primary" on:click=move |_| {
                let value = message.get();
                spawn_local(async { let _ = send_message(value).await; });
                set_message("".into());
            }>"Send"</button>
        </div>
    }
}

#[server]
pub async fn send_message(msg: String) -> Result<(), ServerFnError> {
    use backend::proto::chat_service_client::ChatServiceClient;
    let mut client = ChatServiceClient::connect("http://[::1]:50051").await?;

    //TODO: Set this up with correct username once login functionality is done
    let request = tonic::Request::new(backend::proto::ChatMessage {
        from: "test".into(),
        msg,
        time: "00:04".into(),
    });

    let response = client.send_msg(request).await?;

    Ok(())
}
