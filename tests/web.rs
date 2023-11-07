//! Test suite for the Web and headless browsers.
#[cfg(target_arch = "wasm32")]
mod test {
    use {
        banyan::{
            banyan_api::{client::Client, models::account::Account},
            banyan_wasm::{
                WasmFsMetadataEntry, TombResult, TombWasm, WasmBucket, WasmBucketKey,
            },
        },
        gloo::console::log,
        js_sys::{Array, Uint8Array},
        std::convert::TryFrom,
        tomb_crypt::prelude::{EcEncryptionKey, PrivateKey, PublicKey},
        wasm_bindgen::JsValue,
        wasm_bindgen_test::*,
    };
    wasm_bindgen_test_configure!(run_in_browser);
    const USAGE_LIMIT: u64 = 53_687_091_200;

    fn js_array(values: &[&str]) -> JsValue {
        let js_array: Array = values.iter().map(|s| JsValue::from_str(s)).collect();

        JsValue::from(js_array)
    }

    pub async fn authenticated_client() -> TombResult<TombWasm> {
        let mut client = Client::new("http://127.0.0.1:3001", "http://127.0.0.1:3002")
            .expect("client creation failed");

        let (account, _signing_key) = Account::create_fake(&mut client)
            .await
            .expect("fake account creation failed");
        assert_eq!(account.id.to_string(), client.subject().unwrap());

        let who_am_i = Account::who_am_i(&mut client)
            .await
            .expect("who_am_i failed");
        assert_eq!(account.id, who_am_i.id);

        Ok(TombWasm::from(client))
    }

    pub async fn create_bucket(
        client: &mut TombWasm,
        initial_bucket_key_pem: String,
    ) -> TombResult<WasmBucket> {
        // Generate a random name
        let bucket_name = random_string(10);
        // Note: this might lint as an error, but it's not
        let bucket = client
            .create_bucket(
                bucket_name.clone(),
                "warm".to_string(),
                "interactive".to_string(),
                initial_bucket_key_pem,
            )
            .await?;
        assert_eq!(bucket.name(), bucket_name);
        assert_eq!(bucket.storage_class(), "warm");
        assert_eq!(bucket.bucket_type(), "interactive");
        Ok(bucket)
    }

    fn random_string(length: usize) -> String {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let bytes = (0..length)
            .map(|_| rng.sample(rand::distributions::Alphanumeric))
            .collect();
        String::from_utf8(bytes).unwrap()
    }

    async fn ecencryption_key_pair() -> (String, String) {
        let private_key = EcEncryptionKey::generate()
            .await
            .expect("cant generate private key");
        let public_key = private_key.public_key().expect("cant generate public key");
        let private_pem =
            String::from_utf8(private_key.export().await.expect("cant export private key"))
                .expect("cant convert bytes to PEM");
        let public_pem =
            String::from_utf8(public_key.export().await.expect("cant export public key"))
                .expect("cant convert bytes to PEM");
        (private_pem, public_pem)
    }

    #[wasm_bindgen_test]
    async fn get_usage() -> TombResult<()> {
        log!("tomb_wasm_test: get_usage()");
        let mut client = authenticated_client().await?;
        let usage = client.get_usage().await?;
        assert_eq!(usage, 0);
        let usage_limit = client.get_usage_limit().await?;
        assert_eq!(usage_limit, USAGE_LIMIT);
        Ok(())
    }

    // TODO: probably for API tests
    #[wasm_bindgen_test]
    async fn mount() -> TombResult<()> {
        log!("tomb_wasm_test: create_bucket_mount()");
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;
        let mount = client.mount(bucket.id().to_string(), private_pem).await?;
        assert!(!mount.locked());
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn share_with() -> TombResult<()> {
        log!("tomb_wasm_test: create_bucket_mount_share_with()");
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;
        let wasm_bucket_key: WasmBucketKey =
            client.create_bucket_key(bucket.id().to_string()).await?;
        assert_eq!(wasm_bucket_key.bucket_id(), bucket.id().to_string());
        assert!(!wasm_bucket_key.approved());
        let mut mount = client.mount(bucket.id().to_string(), private_pem).await?;
        assert!(!mount.locked());
        mount.share_with(wasm_bucket_key.id()).await?;
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn snapshot() -> TombResult<()> {
        log!("tomb_wasm_test: create_bucket_mount_snapshot()");
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;
        let mut mount = client
            .mount(bucket.id().to_string(), private_pem.clone())
            .await?;
        assert!(!mount.locked());
        assert!(!mount.has_snapshot());
        let _snapshot_id = mount.snapshot().await?;
        assert!(mount.has_snapshot());
        //assert_eq!(snapshot.bucket_id(), bucket.id().to_string());
        //assert_eq!(
        //    snapshot.metadata_id(),
        //    mount.metadata().expect("metadata").id().to_string()
        //);

        let mount = client
            .mount(bucket.id().to_string(), private_pem.clone())
            .await?;
        assert!(!mount.locked());
        assert!(mount.has_snapshot());
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn mkdir() -> TombResult<()> {
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;

        let mut mount = client.mount(bucket.id().to_string(), private_pem).await?;
        assert!(!mount.locked());

        let mkdir_path_array: Array = js_array(&["test-dir"]).into();
        mount.mkdir(mkdir_path_array).await?;

        let ls_path_array: Array = js_array(&[]).into();
        let ls = mount.ls(ls_path_array).await?;
        assert_eq!(ls.length(), 1);

        let ls_0 = ls.get(0);
        let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();

        assert_eq!(fs_entry.name(), "test-dir");
        assert_eq!(fs_entry.entry_type(), "dir");

        Ok(())
    }

    #[wasm_bindgen_test]
    async fn mkdir_remount() -> TombResult<()> {
        log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls()");
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;

        log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): create_bucket()");
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;
        let mut mount = client
            .mount(bucket.id().to_string(), private_pem.clone())
            .await?;
        assert!(!mount.locked());

        log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): mkdir() and ls()");
        let mkdir_path_array: Array = js_array(&["test-dir"]).into();
        let ls_path_array: Array = js_array(&[]).into();
        mount.mkdir(mkdir_path_array).await?;
        let ls: Array = mount.ls(ls_path_array.clone()).await?;
        assert_eq!(ls.length(), 1);
        log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): remount() and ls()");
        let mut mount = client.mount(bucket.id().to_string(), private_pem).await?;
        assert!(!mount.locked());
        let ls: Array = mount.ls(ls_path_array).await?;
        assert_eq!(ls.length(), 1);
        let ls_0 = ls.get(0);
        let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
        assert_eq!(fs_entry.name(), "test-dir");
        assert_eq!(fs_entry.entry_type(), "dir");
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn write() -> TombResult<()> {
        log!("tomb_wasm_test: create_bucket_mount_mkdir()");
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;
        let mut mount = client.mount(bucket.id().to_string(), private_pem).await?;
        assert!(!mount.locked());
        let write_path_array: Array = js_array(&["zero.bin"]).into();
        let ls_path_array: Array = js_array(&[]).into();
        let zero_content_buffer = Uint8Array::new_with_length(10);
        let zero_content_array_buffer = zero_content_buffer.buffer();
        mount
            .write(write_path_array, zero_content_array_buffer)
            .await?;
        let ls: Array = mount.ls(ls_path_array).await?;
        assert_eq!(ls.length(), 1);
        let ls_0 = ls.get(0);
        let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
        assert_eq!(fs_entry.name(), "zero.bin");
        assert_eq!(fs_entry.entry_type(), "file");
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn write_read() -> TombResult<()> {
        log!("tomb_wasm_test: create_bucket_mount_mkdir()");
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;
        let mut mount = client.mount(bucket.id().to_string(), private_pem).await?;
        assert!(!mount.locked());
        let write_path_array: Array = js_array(&["zero.bin"]).into();
        let ls_path_array: Array = js_array(&[]).into();
        let zero_content_buffer = Uint8Array::new_with_length(1024 * 1024);
        let zero_content_array_buffer = zero_content_buffer.buffer();
        mount
            .write(write_path_array.clone(), zero_content_array_buffer.clone())
            .await?;
        let ls: Array = mount.ls(ls_path_array).await?;
        assert_eq!(ls.length(), 1);
        let ls_0 = ls.get(0);
        let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
        assert_eq!(fs_entry.name(), "zero.bin");
        assert_eq!(fs_entry.entry_type(), "file");
        let new_bytes = mount.read_bytes(write_path_array, None).await?.to_vec();
        // Assert successful reconstruction
        assert_eq!(new_bytes, zero_content_buffer.to_vec());

        Ok(())
    }

    #[wasm_bindgen_test]
    async fn write_remount() -> TombResult<()> {
        log!("tomb_wasm_test: create_bucket_mount_write_ls_remount_ls()");
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;

        log!("tomb_wasm_test: create_bucket_mount_write_ls_remount_ls(): create_bucket()");
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;
        let mut mount = client
            .mount(bucket.id().to_string(), private_pem.clone())
            .await?;
        assert!(!mount.locked());

        log!("tomb_wasm_test: create_bucket_mount_write_ls_remount_ls(): write() and ls()");
        let write_path_array: Array = js_array(&["zero.bin"]).into();
        let ls_path_array: Array = js_array(&[]).into();
        let zero_content_buffer = Uint8Array::new_with_length(10);
        let zero_content_array_buffer = zero_content_buffer.buffer();
        mount
            .write(write_path_array, zero_content_array_buffer)
            .await?;
        mount.mkdir(js_array(&["cats"]).into()).await?;
        let ls: Array = mount.ls(ls_path_array.clone()).await?;
        assert_eq!(ls.length(), 2);

        log!("tomb_wasm_test: create_bucket_mount_write_ls_remount_ls(): remount() and ls()");
        let mut mount = client.mount(bucket.id().to_string(), private_pem).await?;
        assert!(!mount.locked());
        let ls: Array = mount.ls(ls_path_array).await?;
        assert_eq!(ls.length(), 2);
        let ls_0 = ls.get(1);
        let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
        assert_eq!(fs_entry.name(), "zero.bin");
        assert_eq!(fs_entry.entry_type(), "file");
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn write_rm() -> TombResult<()> {
        log!("tomb_wasm_test: create_bucket_mount_write_rm()");
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;
        let mut mount = client.mount(bucket.id().to_string(), private_pem).await?;
        assert!(!mount.locked());
        let write_path_array: Array = js_array(&["zero.bin"]).into();
        let ls_path_array: Array = js_array(&[]).into();
        let zero_content_buffer = Uint8Array::new_with_length(10);
        let zero_content_array_buffer = zero_content_buffer.buffer();
        mount
            .write(write_path_array, zero_content_array_buffer)
            .await?;
        let ls: Array = mount.ls(ls_path_array.clone()).await?;
        assert_eq!(ls.length(), 1);
        let rm_path_array: Array = js_array(&["zero.bin"]).into();
        mount.rm(rm_path_array).await?;
        let ls: Array = mount.ls(ls_path_array).await?;
        assert_eq!(ls.length(), 0);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn write_mv() -> TombResult<()> {
        log!("tomb_wasm_test: create_bucket_mount_write_mv()");
        let mut client = authenticated_client().await?;
        let (private_pem, initial_bucket_key_pem) = ecencryption_key_pair().await;
        let bucket = create_bucket(&mut client, initial_bucket_key_pem).await?;
        let mut mount = client.mount(bucket.id().to_string(), private_pem).await?;
        assert!(!mount.locked());
        let write_path_array: Array = js_array(&["zero.bin"]).into();
        let ls_path_array: Array = js_array(&[]).into();
        let zero_content_buffer = Uint8Array::new_with_length(10);
        let zero_content_array_buffer = zero_content_buffer.buffer();
        mount
            .write(write_path_array, zero_content_array_buffer)
            .await?;
        let ls: Array = mount.ls(ls_path_array.clone()).await?;
        assert_eq!(ls.length(), 1);
        let mv_from_path_array: Array = js_array(&["zero.bin"]).into();
        let mv_to_path_array: Array = js_array(&["zero-renamed.bin"]).into();
        mount.mv(mv_from_path_array, mv_to_path_array).await?;
        let ls: Array = mount.ls(ls_path_array).await?;
        assert_eq!(ls.length(), 1);
        let ls_0 = ls.get(0);
        let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
        assert_eq!(fs_entry.name(), "zero-renamed.bin");
        assert_eq!(fs_entry.entry_type(), "file");
        Ok(())
    }
}
