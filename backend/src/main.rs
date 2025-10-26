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
        println!("[join] Method called");
        let new_user = request.into_inner();
        println!("[join] User: {:?}", new_user);
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
        println!("[send_msg] Method called");
        let msg = dbg!(request.into_inner());
        let mut observers = self.messages.lock().await;

        // Send to all observers and collect indices of failed sends
        let mut failed_indices = Vec::new();
        for (idx, observer) in observers.iter().enumerate() {
            let observer = Arc::clone(&observer);
            let msg = msg.clone();

            // Try to send, mark for removal if channel is closed
            if observer.lock().await.send(Ok(msg)).await.is_err() {
                failed_indices.push(idx);
            }
        }

        // Remove disconnected observers (iterate in reverse to maintain indices)
        for idx in failed_indices.into_iter().rev() {
            observers.remove(idx);
            println!("[send_msg] Removed disconnected receiver at index {}", idx);
        }

        Ok(tonic::Response::new(backend::proto::Empty {}))
    }

    /// Returns a stream of chat messages. Receivers are automatically cleaned up
    /// when clients disconnect (handled in send_msg).
    async fn recieve_msg(
        &self,
        _request: tonic::Request<backend::proto::Empty>,
    ) -> tonic::Result<tonic::Response<Self::RecieveMsgStream>> {
        println!("[recieve_msg] Method called");
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
        println!("[get_all_users] Method called");
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
