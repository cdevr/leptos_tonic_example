use futures::lock::Mutex;
use std::sync::Arc;
use tonic::transport::Server;

use proto::chat_service_server::ChatService;
use tokio_stream::wrappers::ReceiverStream;

use crate::proto::chat_service_server::ChatServiceServer;

mod proto {
    tonic::include_proto!("chat");
}

#[derive(Default)]
struct Chat {
    user_list: Mutex<proto::UserList>,
    messages: Mutex<
        Vec<Arc<Mutex<tokio::sync::mpsc::Sender<Result<proto::ChatMessage, tonic::Status>>>>>,
    >,
}

#[tonic::async_trait]
impl ChatService for Chat {
    async fn join(
        &self,
        request: tonic::Request<proto::User>,
    ) -> std::result::Result<tonic::Response<proto::JoinResponse>, tonic::Status> {
        let new_user = request.into_inner();
        let mut user_list = self.user_list.lock().await;
        let mut response = None;

        if user_list
            .users
            .iter()
            .find(|existing_user| existing_user.name == new_user.name)
            .is_none()
        {
            user_list.users.push(new_user);
            let _ = response.insert(tonic::Response::new(proto::JoinResponse {
                error: 0,
                msg: String::from("Success"),
            }));
        } else {
            let _ = response.insert(tonic::Response::new(proto::JoinResponse {
                error: 1,
                msg: String::from("User already exists."),
            }));
        };

        response.ok_or(tonic::Status::unknown("Error couldn't be determined"))
    }

    async fn send_msg(
        &self,
        request: tonic::Request<proto::ChatMessage>,
    ) -> std::result::Result<tonic::Response<proto::Empty>, tonic::Status> {
        let msg = request.into_inner();
        let observers = self.messages.lock().await;
        // let mut observers = tokio_stream::iter(observers.iter());

        observers.iter().for_each(|observer| {
            let observer = Arc::clone(&observer);
            let msg = msg.clone();
            tokio::spawn(async move {
                observer.lock().await.send(Ok(msg)).await.unwrap();
            });
        });

        Ok(tonic::Response::new(proto::Empty {}))
    }

    async fn recieve_msg(
        &self,
        _request: tonic::Request<proto::Empty>,
    ) -> std::result::Result<tonic::Response<Self::RecieveMsgStream>, tonic::Status> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1000);

        self.messages
            .lock()
            .await
            .push(Arc::new(Mutex::new(sender)));

        Ok(tonic::Response::new(ReceiverStream::new(receiver)))
        // Err(tonic::Status::unimplemented(""))
    }

    async fn get_all_users(
        &self,
        _request: tonic::Request<proto::Empty>,
    ) -> Result<tonic::Response<proto::UserList>, tonic::Status> {
        let user_list = self.user_list.lock().await;
        Ok(tonic::Response::new(user_list.clone()))
    }

    type RecieveMsgStream = ReceiverStream<Result<proto::ChatMessage, tonic::Status>>;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();
    let chat_service = Chat::default();

    println!("ChatServer listening on: {}", addr);

    Server::builder()
        .add_service(ChatServiceServer::new(chat_service))
        .serve(addr)
        .await?;
    Ok(())
}
