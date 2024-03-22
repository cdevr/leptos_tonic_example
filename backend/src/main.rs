use futures::lock::Mutex;
use std::sync::Arc;
use tonic::transport::Server;

use backend::proto::chat_service_server::ChatService;
use tokio_stream::wrappers::ReceiverStream;

use backend::proto::chat_service_server::ChatServiceServer;

#[derive(Default)]
struct Chat {
    user_list: Mutex<backend::proto::UserList>,
    messages: Mutex<
        Vec<Arc<Mutex<tokio::sync::mpsc::Sender<tonic::Result<backend::proto::ChatMessage>>>>>,
    >,
}

#[tonic::async_trait]
impl ChatService for Chat {
    async fn join(
        &self,
        request: tonic::Request<backend::proto::User>,
    ) -> tonic::Result<tonic::Response<backend::proto::JoinResponse>> {
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
            let _ = response.insert(tonic::Response::new(backend::proto::JoinResponse {
                error: 0,
                msg: String::from("Success"),
            }));
        } else {
            let _ = response.insert(tonic::Response::new(backend::proto::JoinResponse {
                error: 1,
                msg: String::from("User already exists."),
            }));
        };

        response.ok_or(tonic::Status::unknown("Error couldn't be determined"))
    }

    async fn send_msg(
        &self,
        request: tonic::Request<backend::proto::ChatMessage>,
    ) -> tonic::Result<tonic::Response<backend::proto::Empty>> {
        let msg = dbg!(request.into_inner());
        let observers = self.messages.lock().await;
        // let mut observers = tokio_stream::iter(observers.iter());

        observers.iter().for_each(|observer| {
            let observer = Arc::clone(&observer);
            let msg = msg.clone();
            tokio::spawn(async move {
                observer.lock().await.send(Ok(msg)).await.unwrap();
            });
        });

        Ok(tonic::Response::new(backend::proto::Empty {}))
    }

    ///TODO: Sending the receiver over to the client works functionally but if the client is refreshed the
    ///reveiver channel lingers. Will need to think of a way to remove expired receivers from the
    ///list.     
    async fn recieve_msg(
        &self,
        _request: tonic::Request<backend::proto::Empty>,
    ) -> tonic::Result<tonic::Response<Self::RecieveMsgStream>> {
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
        _request: tonic::Request<backend::proto::Empty>,
    ) -> Result<tonic::Response<backend::proto::UserList>, tonic::Status> {
        let user_list = self.user_list.lock().await;
        Ok(tonic::Response::new(user_list.clone()))
    }

    type RecieveMsgStream = ReceiverStream<tonic::Result<backend::proto::ChatMessage>>;
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
