

trait ApiRequest {
    // Obtain the url suffix of the endpoint
    fn get_endpoint() -> String;
    
}


/// Sub-commands associated with configuration
#[derive(Clone, Debug)]
pub enum Request {
    /// Set the remote endpoint where buckets are synced to / from
    Bucket {
        subcommand: BucketRequest
    },

    /// Set the remote endpoint where buckets are synced to / from
    Keys {
        subcommand: KeyRequest
    },

    /// Set the remote endpoint where buckets are synced to / from
    Metadata {
        subcommand: MetadataRequest
    },
}


/// Sub-commands associated with configuration
#[derive(Clone, Debug)]
pub enum BucketRequest {
    Create {
        name: String,
    },
    List {

    },
    Get {

    },
    Delete {

    }
}


/// Sub-commands associated with configuration
#[derive(Clone, Debug)]
pub enum KeyRequest {
    Create {
        
    },
    Get {

    },
    Delete {

    },
}


/// Sub-commands associated with configuration
#[derive(Clone, Debug)]
pub enum MetadataRequest {
    Create {
        
    },
    Get {

    },
    Delete {

    },
}
