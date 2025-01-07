pub mod sf {
    pub mod bstream {
        pub mod v1 {
            include!("sf.bstream.v1.rs");
        }
    }
}
pub mod google {
    pub mod protobuf {
        include!("google.protobuf.rs");
    }
}
