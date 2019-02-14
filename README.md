## rust_iron_rest
Simple rest service for image upload created with iron framework.

How to use:

1. Create storage for your uploads with
```
mkdir -p /tmp/images/thumbs/
```
2. Run server with
```
cargo run
```
or with 
```
cargo run --features cvresize
```
to test it with opencv variant of resize function. (Need OpenCV 3.2 installed)
3. Test service with scripts provided
- **postimg.sh** - contains both base64 and url variants, both valid.
- **postimg_bad_base64.sh** - same with invalid base64 data.
- **postimg_bad_image.sh** - same with invalid data stored as base64.
- **postimg_bad_url.sh** - no base64 images, just invalid url.
