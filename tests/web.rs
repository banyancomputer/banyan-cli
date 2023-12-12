//! Test suite for the Web and headless browsers.
#[cfg(target_arch = "wasm32")]
mod test {
    use {
        banyan_cli::prelude::{
            api::{client::Client, models::account::Account},
            blockstore::MemoryBlockStore,
            filesystem::{sharing::SharedFile, FilesystemError, FsMetadata},
            wasm::{
                register_log, TombResult, TombWasm, WasmBucket, WasmBucketKey, WasmBucketMount,
                WasmFsMetadataEntry,
            },
        },
        js_sys::{Array, Uint8Array},
        std::convert::TryFrom,
        tomb_crypt::prelude::{EcEncryptionKey, PrivateKey, PublicKey},
        tracing::info,
        wasm_bindgen::{convert::TryFromJsValue, JsValue},
        wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure},
    };
    wasm_bindgen_test_configure!(run_in_browser);
    const USAGE_LIMIT: u64 = 53_687_091_200;

    fn js_array(values: &[&str]) -> JsValue {
        let js_array: Array = values.iter().map(|s| JsValue::from_str(s)).collect();

        JsValue::from(js_array)
    }

    pub async fn authenticated_client() -> TombResult<TombWasm> {
        register_log();
        let mut client = Client::new("http://127.0.0.1:3001").expect("client creation failed");

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

    pub async fn create_bucket_and_mount(
        client: &mut TombWasm,
        private_pem: String,
        public_pem: String,
    ) -> TombResult<WasmBucketMount> {
        // Generate a random name
        let bucket_name = random_string(10);
        // Note: this might lint as an error, but it's not
        let bucket_mount = client
            .create_bucket_and_mount(
                bucket_name.clone(),
                "warm".to_string(),
                "interactive".to_string(),
                private_pem,
                public_pem,
            )
            .await?;
        let bucket = bucket_mount.bucket();
        assert_eq!(bucket.name(), bucket_name);
        assert_eq!(bucket.storage_class(), "warm");
        assert_eq!(bucket.bucket_type(), "interactive");
        Ok(bucket_mount)
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

    // TODO: probably for API tests
    #[wasm_bindgen_test]
    async fn create_and_mount() -> TombResult<()> {
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount = create_bucket_and_mount(&mut client, private_pem, public_pem).await?;
        assert!(!bucket_mount.mount().locked());
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn get_usage() -> TombResult<()> {
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: get_usage()");
        let usage = client.get_usage().await?;
        assert_eq!(usage, 0);
        let usage_limit = client.get_usage_limit().await?;
        assert_eq!(usage_limit, USAGE_LIMIT);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn rename() -> TombResult<()> {
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_rename()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount = create_bucket_and_mount(&mut client, private_pem, public_pem).await?;
        client
            .rename_bucket(
                bucket_mount.bucket().id().to_string(),
                "new_name".to_string(),
            )
            .await?;
        let buckets = client.list_buckets().await?;
        let bucket = WasmBucket::try_from_js_value(buckets.get(0)).unwrap();
        assert_eq!(bucket.name(), "new_name");
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn mount_rename() -> TombResult<()> {
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_rename()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount =
            create_bucket_and_mount(&mut client, private_pem.clone(), public_pem).await?;
        let mut mount = bucket_mount.mount();
        mount.rename("new_name".to_string()).await?;
        assert_eq!(mount.bucket().name(), "new_name");
        let mount = client
            .mount(bucket_mount.bucket().id().to_string(), private_pem)
            .await?;
        assert_eq!(mount.bucket().name(), "new_name");
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn share_with() -> TombResult<()> {
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_share_with()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount =
            create_bucket_and_mount(&mut client, private_pem.clone(), public_pem).await?;
        let bucket = bucket_mount.bucket();
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
    async fn write_share_receive_local() -> Result<(), FilesystemError> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let mut fs_metadata = FsMetadata::init(&wrapping_key).await?;
        fs_metadata.save(&metadata_store, &content_store).await?;
        fs_metadata = FsMetadata::unlock(wrapping_key, &metadata_store).await?;

        let cat_path = vec!["cat.txt".to_string()];
        let kitty_bytes = "hello kitty".as_bytes().to_vec();
        // Add a new file
        fs_metadata
            .write(
                &cat_path,
                &metadata_store,
                &content_store,
                kitty_bytes.clone(),
            )
            .await?;

        let shared_file = fs_metadata.share_file(&cat_path, &content_store).await?;

        let share_string = shared_file.export_b64_url()?;

        let reconstructed_shared_file = SharedFile::import_b64_url(share_string)?;

        let new_kitty_bytes =
            FsMetadata::receive_file_content(reconstructed_shared_file, &content_store).await?;

        assert_eq!(kitty_bytes, new_kitty_bytes);

        Ok(())
    }

    #[wasm_bindgen_test]
    async fn mount_write_share_receive_local() -> TombResult<()> {
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_write_share_receive()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount =
            create_bucket_and_mount(&mut client, private_pem.clone(), public_pem).await?;
        let mut mount = bucket_mount.mount();
        assert!(!mount.locked());
        let write_path_array: Array = js_array(&["zero.bin"]).into();
        let ls_path_array: Array = js_array(&[]).into();
        let zero_content_buffer = Uint8Array::new_with_length(10);
        let zero_content_array_buffer = zero_content_buffer.buffer();
        mount
            .write(write_path_array.clone(), zero_content_array_buffer.clone())
            .await?;
        let ls: Array = mount.ls(ls_path_array.clone()).await?;
        assert_eq!(ls.length(), 1);
        let ls_0 = ls.get(0);
        let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
        assert_eq!(fs_entry.name(), "zero.bin");
        assert_eq!(fs_entry.entry_type(), "file");
        let shared_string = mount.share_file(write_path_array).await?;

        let cs = mount.content_blockstore();
        let new_bytes = client
            .read_shared_file_from_bs(shared_string, &cs)
            .await?
            .to_vec();
        // Assert successful reconstruction
        assert_eq!(new_bytes, zero_content_buffer.to_vec());

        Ok(())
    }

    #[wasm_bindgen_test]
    async fn snapshot() -> TombResult<()> {
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_snapshot()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount =
            create_bucket_and_mount(&mut client, private_pem.clone(), public_pem).await?;
        let mut mount = bucket_mount.mount();
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
            .mount(mount.bucket().id().to_string(), private_pem)
            .await?;
        assert!(!mount.locked());
        assert!(mount.has_snapshot());
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn mkdir() -> TombResult<()> {
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: mkdir()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount = create_bucket_and_mount(&mut client, private_pem, public_pem).await?;
        let mut mount = bucket_mount.mount();
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
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;

        info!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): create_bucket_and_mount()");
        let bucket_mount =
            create_bucket_and_mount(&mut client, private_pem.clone(), public_pem).await?;
        let mut mount = bucket_mount.mount();
        assert!(!mount.locked());

        info!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): mkdir() and ls()");
        let mkdir_path_array: Array = js_array(&["test-dir"]).into();
        let ls_path_array: Array = js_array(&[]).into();
        mount.mkdir(mkdir_path_array).await?;
        let ls: Array = mount.ls(ls_path_array.clone()).await?;
        assert_eq!(ls.length(), 1);
        info!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): remount() and ls()");
        let mut mount = client
            .mount(bucket_mount.bucket().id().to_string(), private_pem)
            .await?;
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
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_mkdir()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount = create_bucket_and_mount(&mut client, private_pem, public_pem).await?;
        let mut mount = bucket_mount.mount();
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
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_mkdir()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount = create_bucket_and_mount(&mut client, private_pem, public_pem).await?;
        let mut mount = bucket_mount.mount();
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
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_write_ls_remount_ls()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;

        info!(
            "tomb_wasm_test: create_bucket_mount_write_ls_remount_ls(): create_bucket_and_mount()"
        );
        let bucket_mount =
            create_bucket_and_mount(&mut client, private_pem.clone(), public_pem).await?;
        let mut mount = bucket_mount.mount();
        assert!(!mount.locked());

        info!("tomb_wasm_test: create_bucket_mount_write_ls_remount_ls(): write() and ls()");
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

        info!("tomb_wasm_test: create_bucket_mount_write_ls_remount_ls(): remount() and ls()");
        let mut mount = client
            .mount(bucket_mount.bucket().id().to_string(), private_pem)
            .await?;
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
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_write_rm()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount = create_bucket_and_mount(&mut client, private_pem, public_pem).await?;
        let mut mount = bucket_mount.mount();
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
        register_log();
        let mut client = authenticated_client().await?;
        info!("tomb_wasm_test: create_bucket_mount_write_mv()");
        let (private_pem, public_pem) = ecencryption_key_pair().await;
        let bucket_mount = create_bucket_and_mount(&mut client, private_pem, public_pem).await?;
        let mut mount = bucket_mount.mount();
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
