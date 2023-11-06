use std::{sync::Mutex, thread, time::Duration};

use cargo_fixture::{with_fixture, TestClient};

pub mod common;
use common::cargo_fixture;

use crate::common::confirm_callback_ran;

#[test]
fn serial() {
    cargo_fixture()
        .exact(false)
        .run_test("serial")
        .output()
        .assert_success();
}

static TEST_LOCK: Mutex<bool> = Mutex::new(true);

macro_rules! serial_callback {
    ($name:ident) => {
        #[with_fixture(serial)]
        #[smol_potat::test]
        async fn $name(_client: TestClient) {
            let mut lock = TEST_LOCK.try_lock().expect("serial test not serial");
            let confirm = *lock;
            *lock = false;
            thread::sleep(Duration::from_millis(20));

            if confirm {
                confirm_callback_ran("serial");
            }
        }
    };
}

macro_rules! serial_callback_nonserial {
    ($name:ident) => {
        #[with_fixture(serial)]
        #[smol_potat::test]
        async fn $name(_client: TestClient) {
            // maybe lock the lock for a bit
            let _lock = TEST_LOCK.try_lock();
            thread::sleep(Duration::from_millis(20));
        }
    };
}

serial_callback!(serial_callback_1);
serial_callback!(serial_callback_2);
serial_callback!(serial_callback_3);
serial_callback!(serial_callback_4);
serial_callback!(serial_callback_5);
serial_callback!(serial_callback_6);
serial_callback!(serial_callback_7);
serial_callback!(serial_callback_8);

serial_callback_nonserial!(serial_callback_nonserial_1);
serial_callback_nonserial!(serial_callback_nonserial_2);
serial_callback_nonserial!(serial_callback_nonserial_3);
serial_callback_nonserial!(serial_callback_nonserial_4);
serial_callback_nonserial!(serial_callback_nonserial_5);
serial_callback_nonserial!(serial_callback_nonserial_6);
serial_callback_nonserial!(serial_callback_nonserial_7);
serial_callback_nonserial!(serial_callback_nonserial_8);
