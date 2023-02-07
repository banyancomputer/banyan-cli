# carfile-rs

- this makes blocks into a carv2 file. it does so efficiently with one write to the car file from whatever buffer you're writing from.
- This is a harder problem than you might think! carfiles require that you list all CIDs inside the CAR file in the header.
- This means you have a variable size header that contains checksums that can only be computed AFTER you've done a pass over the data.
- So like... do you write everything twice? Do you loop over all the data twice? No! you keep two pointers into the file and leave some whitespace..
- This library creates 32 gigabyte car files for Filecoin.
- It leaves a megabyte of space up top to put CIDs in. That's a lot of CIDs, and more than enough to hold 16000 2MB blocks and completely fill the 32G CAR filecoin piece.
- So even if you have a lot of little blocks, you can still fill it.
- It also fills and indexes multiple car files for you as you go.
- the entrance point is the BlockStore trait from wnfs
```rust
vroom(byte_streams: BoxStream<Block>) -> BoxStream<(CarFilePath, Cid)>
```
