//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
// use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test_configure;
use wasm_bindgen_test::*;

extern crate tomb_wasm;
// use tomb_wasm::Tomb;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn pass() {
    assert_eq!(1 + 1, 2);
}

// #[cfg(test)]
// pub async fn helper_method_example() -> Tomb {
//     // TODO: Add real endpoint and token
//     let tomb = Tomb::new(
//         "http://test.tomb.local".to_string(),
//         "long-secure-token-here".to_string(),
//     )
//     .await
//     .unwrap();
//     tomb
// }

// #[wasm_bindgen_test]
// pub async fn test_tomb() {
//     // calling a setup function.
//     let mut tomb = helper_method_example().await;
//     let _buckets = tomb.buckets().unwrap();
//     let pkcs8_string: &str = "MIIG/wIBADANBgkqhkiG9w0BAQEFAASCBukwggblAgEAAoIBgQCmCzlOSlcOqPJ7YqhTURG7xBGzyUeQlv9hQuWc9Hcm4BR92mPpPCJ8tlMUXcO6HkJFRcRrSnfsTNFoDetcJ8FKV0PiybW5nzGlz7CU1PRm59EdjY9oWt5cQIOjUe0UyxoHp4YoXYpjdGFiWpSuZRiiabcz8byoEbVOe8gJknEW4KnMTtauHcHHDkFHO5yVpq2lrducn/o0OYGoWxm9qeWsoqhJ0qxr+3bvhReha4AW2NG6Zslie7ujdCRXAWMJERd3agNW70SeupEevXXMT0ofzCHaqTR1uWI9e8H0GXAFkcuFZ3tuEK12F/xP61ZV+muQzviH3VJF9lMtrroHg9RymG17iDDiEJpuhkh50sbQSfjmyEyXf+tiH5gRigknXyoHOIVQyr4KtbNM+690zJYGtuKbLxMSTNHzdRfCmdw+QOHWokkfLDo8aGFfKJg+03BeeuSvkGXU1Gzet9uLtGTiZtNkn3YsW1sM0Chmdlc2aur5lBn+UslSp3UbbG3XJoUCAwEAAQKCAYARKMxHibm092M1upScJZ7gSWst6gFmESC7t6rcfUwZ/aLIfcsA9bi3rCzqSCVbxNhC6eqaTuQVTLwAVZ3q1GXujZWjqIZJ9EhwcwXz340RXGgZNoGpPmjH3lfsRyFp2nJqc5bS8ZXFYOfWfvdqDWMOF8A500PUl53lyjd6O8LJozaQ+V3IuSUHMfMvjhrIwWSlIFI3fbXg80dxs1Z16gqk/FtJY8bzUtWv+5BdW2ttkQMdkRVDQve5dN1zi15ld7lLNgv2OXap7d5M3PBQumP6gmSIplu3mgC3lhkGnxX6/k7aTynsZrxcNk6RlGHFiCTTuvOXl4C6yCmPwUGdGs8CPFTrKKYkylfWkJgRioaoCvGNwQPkCkkXmmToNnPECvOty9nW2y0utp6B0KgwEE1Wy5+uiCixRQpDqdK3QJBzba02q7PTtJG7kaBrwrl+w+DDbsqg5aPZRluZVTG1xMe6SAqFQ+qexBklUinUHkrW/QWa9LULr32WwlJLdHm+W/kCgcEA1kV4w2znWPFedgWBS0IcadgqkgIaSL4qh+2HW3+jAUNaXgXtWg+kSHaEJjp7H3FD/90Fg/EhTFo/ZPdqTfhTjkKbWON+DHixts6wC8+MyRU+LP0p+RK1syEFcpvaO2rzfYlg3PJYAhBt65wLaTeHNPclluTKqgAjAuj6cWaMLUvfkkbFU/hd/nrG1U+t/c5j3TV/HpgRDWja3A4zxYOWFu48l4lWeH7MNl5Yvh1cDCHPYwKr/u1XIl1oqKpVP3jtAoHBAMZhXLAgI79OlvVKE9UxUzXvKfXoCSO4yLq2bs51n7GB3P+AxI2FMq7ZIGYh76y8Jm1zgq0r4Q7k8wZ57nvewB4lCTe0O1YqZHRhs+Kgf7dygeg3iTO0ijvQOM62i28MyHzLMXdekouzWiJd36Uq4q+UnHAgPg2mXlhxVr1g8mIC3bi7nh+5WSHqUMnQ2rNFRHkMPjhoSmM6NdJwikiFNkjsdWApssd67Xz9+zqJzKv8rPPj6lved3FQyMAG7duo+QKBwQCzQ+ArL/vF7/plp2lqu17mNtI24cd3wJH4swMhzAFmVyFNtIvFY3zAm1coXJkRz0Ni11l778s6A+8x28V2giH1zUgG8B1O9dNI7FdhKj3RJhKktRHeroaR3TifkEDeoTYhe0Qs1hxHbdNo4V6yoqBd8b/jJHtiC0c/cgfFxFPWubnMuaTyAcMx2ypq4ITi6T+nnNBDmln57BXfMYqi3to9SQgsh9xuZzcW7Yw1Un7mL4tAfMXFPHA/8gJTyl4UAmkCgcEAmEB9HIduKBMu9I6n7gVvMYOelqZA7XOSSwpcvIO1zkw2yrmPIHZL0bm+jeQZyF6Wt4XhkvqMPhwlEKFgER2CISCXlHL030ql0lRx9MrtemOdpBWLbW1wcjt6fdvH47DR5kUkb9LbcfByitG1JVRmqg7KiZuVRHCdFA/YXHwdSm+cr3z+/KYJ7GejHWD3mILe7HAjCLOx87nnON06pDHo2crwwp7+IO8NedKLj//WX2ELdBtF8MAqt4Mir44h22YxAoHBAIjZGFLXxN/3n6BjO2QuCy8N5QT+REEKUluKs5ne2RQJaryEWvesIgaWFjl2p8ZNJeJwOsviiizQmvcDbCrhS2U5hcZbH8/+pnkGec0k5gqbd0KjP4ZLVf3hebEzYqKV2JF1Q7Ac0yHh/Z9NJJEG1qKb0xbitIm2fu0FEvxfI/r4eTZDZ4iq8M4HTXKAqP+31Oe/8wnJHLPTu7EckgN6/+kAmvXbufVuKoJ1JukcjAp1AJYyemacI2YuqPaZtNbgFw==";
//     let js_value_pkcs8_string: JsValue = wasm_bindgen::JsValue::from_str(pkcs8_string);
//     tomb.load_bucket("bucket_name".to_string(), js_value_pkcs8_string)
//         .await
//         .unwrap();
//     // TODO: More in depth assertions
// }
