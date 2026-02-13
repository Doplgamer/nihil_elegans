use hickory_proto::op::{Header, ResponseCode};
use hickory_proto::rr::rdata::A;
use hickory_proto::rr::{RData, Record, RecordType};
use hickory_server::authority::MessageResponseBuilder;
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};
use tokio::sync::mpsc::Sender;
use shared::{Action, decrypt};

// TODO make connecting optionally locked behind a password
pub struct MyHandler {
    sender: Sender<Action>,
    xor_key: String,
}

impl MyHandler {
    pub fn new(sender: Sender<Action>, xor_key: String) -> Self {
        Self {
            sender,
            xor_key
        }
    }
}

#[async_trait::async_trait]
impl RequestHandler for MyHandler {
    async fn handle_request<R: ResponseHandler>(&self, request: &Request, mut response_handle: R) -> ResponseInfo {
        let message = request.queries()[0].original();
        let message_name = message.name().clone();

        self.sender.send(
            Action::Log(
                format!("Decrypting message: {}",
                        String::from_utf8_lossy(
                            &decrypt(vec![message_name.to_string().to_uppercase()], self.xor_key.as_bytes())
                            .unwrap_or("Error: Could not decrypt".into())
                        )
                ))).await.ok();
        
        let builder = MessageResponseBuilder::from_message_request(request);

        if let RecordType::A = message.query_type {
            let answer_a = A::new(4, 20, 69, 67);
            let answer = Record::from_rdata(
                message_name,
                60,
                RData::A(answer_a)
            );

            let response_msg = builder.build(Header::response_from_request(request.header()),
                                             vec![&answer],
                                             vec![],
                                             vec![],
                                             vec![]);

            response_handle.send_response(response_msg).await.unwrap()
        } else {
            let err_msg = builder.error_msg(
                &Header::response_from_request(request.header()),
                ResponseCode::Refused);

            response_handle.send_response(err_msg).await.unwrap()
        }
    }
}

// If client has not been registered, merely echo the domain name back in a TXT field (nvm this is for after we get base32 working)
// Just send queries in base32 [data].domain.com, worry about encryption later