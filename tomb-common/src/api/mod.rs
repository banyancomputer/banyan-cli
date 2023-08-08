mod request;
mod client;
mod token;
mod error;
mod credentials;

#[cfg(test)]
mod test {
    use tomb_crypt::prelude::EcEncryptionKey;

    // pub async fn register_fake_account() -> (Uuid, EcEncryptionKey) {
    //     let mut api_client = ClientBuilder::default().build().expect("client");

    //     let account_info = api_client
    //         .call(fake::RegisterFakeAccount)
    //         .await
    //         .unwrap();

    //     let private_pem = fake::create_private_ec_pem();
    //     let public_pem = fake::public_from_private(&private_pem);

    //     let device_key_info = api_client
    //         .call(fake::FakeRegisterDeviceKey {
    //             token: account_info.token,
    //             public_key: public_pem.clone(),
    //         })
    //         .await
    //         .unwrap();

    //     let fingerprint = fake::fingerprint_public_pem(public_pem.as_str());

    //     assert_eq!(account_info.id, device_key_info.account_id);
    //     assert_eq!(fingerprint, device_key_info.fingerprint);

    //     Account {
    //         id: account_info.id,
    //         device_private_key_pem: private_pem,
    //         fingerprint,
    //     }
    // }

    // pub async fn fake_authenticated_client() -> Client {
    //     let account = register_fake_account().await;
    //     let jwt_signing_key =
    //         EncodingKey::from_ec_pem(account.device_private_key_pem.as_bytes()).unwrap();
    //     let mut api_client = ClientBuilder::default().build().expect("client");
    //     api_client.set_credentials(account.id, account.fingerprint, jwt_signing_key);
    //     // Query who the API thinks we're authenticated as
    //     let authenticated_info = api_client.call(WhoAmI).await.unwrap();
    //     // Assert that it's us!
    //     assert_eq!(authenticated_info.account_id, account.id);
    //     // Return client
    //     api_client
    // }
}