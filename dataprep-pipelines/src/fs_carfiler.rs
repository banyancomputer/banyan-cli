// DEPRECATED until need / design for CAR filing is determined

// Note (laudiacay): i think i'm willing to bet that the way people wanna make a car file is "put as large a subtree as you can into each one"
// so we're gonna start at the leftmost node. we're gonna try and get as many of its siblings as we can.
// if we succeed we're gonna go up a node. and get as many of its siblings as we can.
// this is... hard.

use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use iroh_car::{CarWriter, CarHeader};
use anyhow::Result;

const MAX_CAR_SIZE: usize = 1024 * 1024 * 1024;

pub struct CarsWriter {
    /// current car writer because we will be making a lot of car files...
    current_car_writer: RefCell<CarWriter>,
    /// space_left_in_current is the amount of space left in the current car file
    space_left_in_current: usize,
    /// we're using the same one for all of them and it's hardcoded. cope
    header: CarHeader,
    /// header_size is how many bytes the header takes up
    header_size: usize,
    /// cars_so_fars is how many of the car files we've opened up so far (zero indexed).
    /// {car_dir}/car_{cars_so_fars}.car is where we're writing to right now!
    cars_so_fars: RefCell<usize>,
    /// car_dir is the directory we're writing the car files to
    car_dir: PathBuf,
}

impl CarsWriter {
    fn new_car_smell(&mut self) {
        self.cars_so_fars += 1;
        let new_car_loc = self.car_dir.join(format!("car_{}.car", *self.cars_so_fars));
        let mut car = CarWriter::new(self.header.clone(), File::open(new_car_loc));
        self.current_car_writer.replace(car);
        self.space_left_in_current = MAX_CAR_SIZE - self.header_size;
    }

    pub fn new(cars_dir: PathBuf) -> Self {
        assert!(cars_dir.is_dir());
        // secretly encodes twice. dumb
        let header = CarHeader::new_v1(vec![Cid::from_str("bafkqaaa").unwrap()]);
        let header_size = header.encode().len();
        let new_car_loc = cars_dir.join(format!("car_{}.car", 0));
        let mut car = CarWriter::new(header, File::open(new_car_loc));
        CarsWriter {
            current_car_writer: RefCell::new(car),
            space_left_in_current: MAX_CAR_SIZE - header_size,
            header,
            header_size,
            cars_so_fars: RefCell::new(0),
            car_dir: cars_dir,
        }
    }

    /// computes the varint and CID for this buf and writes it to the current car file
    /// if there's not enough space in the current car file, it opens a new one
    /// and writes the varint and CID to that one after writing the header :)
    async fn write_block_raw(&mut self, buf: &[u8]) -> Result<usize> {
        // compute CID of buf
        let digest = multihash::Code::Sha2_256.digest(buf);
        let cid = Cid::new_v1(Cid::SHA2_256, digest);
        // compute the varint size of the cid + buf
        let cid_size = cid.to_bytes().len();
        let buf_size = buf.len();
        let varint = varint::encode(cid_size + buf_size);
        let varint_size = varint.len();

        if buf.len() + cid_size + varint_size > self.space_left_in_current {
            self.new_car_smell();
        };

        *self.current_car_writer.write(varint).await;
        *self.current_car_writer.write(cid.to_bytes()).await;
        *self.current_car_writer.write(buf).await;
        self.space_left_in_current -= buf.len() + cid_size + varint_size;
        Ok(buf.len())
    }

    async fn finish(mut self) -> Result<()> {
        self.current_car_writer.flush().await?;
        Ok(())
    }
    async fn flush (&mut self) -> Result<()> {
        self.current_car_writer.flush().await?;
        Ok(())
    }
}
