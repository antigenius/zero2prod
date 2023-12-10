use lettre::{Message, SmtpTransport, Transport};
use lettre::message::MultiPart;
use lettre::transport::smtp::authentication::Credentials;
use secrecy::{Secret, ExposeSecret};

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    credentials: Credentials,
    host: String,
    sender: SubscriberEmail,
}

impl EmailClient {
    pub fn new(
        sender: SubscriberEmail,
        username: Secret<String>,
        password: Secret<String>,
        host: String,
    ) -> Self {
        let credentials = Credentials::new(
            username.expose_secret().to_string().clone(),
            password.expose_secret().to_string().clone()
        );

        Self {
            credentials,
            host,
            sender,
        }
    }
    
    #[tracing::instrument(
        name = "Sending an email.",
        skip(self, recipient, subject, html, plain)
        fields(
            subscriber_email = %recipient
        )
    )]
    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html: &str,
        plain: &str
    ) -> Result<<SmtpTransport as Transport>::Ok, <SmtpTransport as Transport>::Error> {
        let sender = self.sender.as_ref().parse().unwrap();
        let recipient = recipient.as_ref().parse().unwrap();
        let part = MultiPart::alternative_plain_html(
            plain.to_string(),
            html.to_string()
        );
        let message = Message::builder()
            .from(sender)
            .to(recipient)
            .subject(subject)
            .multipart(part)
            .unwrap();
        let sender = SmtpTransport::builder_dangerous(&self.host)
            .port(1025)
            .credentials(self.credentials.clone())
            .build();

        sender.send(&message)
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}


//#[cfg(test)]
// mod tests {
//     use claims::{assert_ok, assert_err};
//     use fake::{Fake, Faker};
//     use fake::faker::internet::en::SafeEmail;
//     use fake::faker::lorem::en::{Paragraph, Sentence};
//     use secrecy::Secret;
//     use wiremock::{Mock, MockServer, Request, ResponseTemplate};
//     use wiremock::matchers::{any, header, header_exists, method, path};

//     use crate::domain::SubscriberEmail;
//     use crate::email_client::EmailClient;


//     fn subject() -> String {
//         Sentence(1..2).fake()
//     }

//     fn content() -> String {
//         Paragraph(1..10).fake()
//     }

//     fn email() -> SubscriberEmail {
//         SubscriberEmail::parse(SafeEmail().fake()).unwrap()
//     }

//     fn email_client(base_url: String) -> EmailClient {
//         EmailClient::new(
//             email(), 
//             Secret::new(Faker.fake()),
//             Secret::new(Faker.fake()),
//             base_url,
//         )
//     }
    
//     struct SendEmailBodyMatcher;

//     impl wiremock::Match for SendEmailBodyMatcher {
//         fn matches(&self, request: &Request) -> bool {
//             let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

//             if let Ok(body) = result {
//                 body.get("From").is_some()
//                     && body.get("To").is_some()
//                     && body.get("Subject").is_some()
//                     && body.get("HtmlBody").is_some()
//                     && body.get("TextBody").is_some()
//             } else {
//                 false
//             }
//         }
//     }

//     #[tokio::test]
//     async fn send_email_sends_the_expected_request() {
//         let mock_server = MockServer::start().await;
//         let email_client = email_client(mock_server.uri());

//         Mock::given(header_exists("X-Postmark-Server-Token"))
//             .and(header("Content-Type", "application/json"))
//             .and(path("/email"))
//             .and(method("POST"))
//             .and(SendEmailBodyMatcher)
//             .respond_with(ResponseTemplate::new(200))
//             .expect(1)
//             .mount(&mock_server)
//             .await;

//         let _ = email_client
//             .send_email(&email(), &subject(), &content(), &content())
//             .await;
//     }

//     #[tokio::test]
//     async fn send_email_succeeds_if_the_server_returns_200() {
//         let mock_server = MockServer::start().await;
//         let email_client = email_client(mock_server.uri());

//         Mock::given(any())
//             .respond_with(ResponseTemplate::new(200))
//             .expect(1)
//             .mount(&mock_server)
//             .await;
        
//         let outcome = email_client
//             .send_email(&email(), &subject(), &content(), &content())
//             .await;

//         assert_ok!(outcome);
//     }

//     #[tokio::test]
//     async fn send_email_fails_if_the_server_returns_500() {
//         let mock_server = MockServer::start().await;
//         let email_client = email_client(mock_server.uri());

//         Mock::given(any())
//             .respond_with(ResponseTemplate::new(500))
//             .expect(1)
//             .mount(&mock_server)
//             .await;
        
//         let outcome = email_client
//             .send_email(&email(), &subject(), &content(), &content())
//             .await;

//         assert_err!(outcome);
//     }

//     #[tokio::test]
//     async fn send_email_times_out_if_the_server_takes_too_long() {
//         let mock_server = MockServer::start().await;
//         let email_client = email_client(mock_server.uri());
//         let response = ResponseTemplate::new(200)
//             .set_delay(std::time::Duration::from_secs(180));

//         Mock::given(any())
//             .respond_with(response)
//             .expect(1)
//             .mount(&mock_server)
//             .await;
        
//         let outcome = email_client
//             .send_email(&email(), &subject(), &content(), &content())
//             .await;

//         assert_err!(outcome);
//     }
// }
