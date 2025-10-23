// Router module

use crate::config::BucketConfig;

pub struct Router {
    #[allow(dead_code)]
    buckets: Vec<BucketConfig>,
}

impl Router {
    pub fn new(buckets: Vec<BucketConfig>) -> Self {
        Router { buckets }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BucketConfig;

    #[test]
    fn test_can_create_router_with_empty_bucket_list() {
        let buckets: Vec<BucketConfig> = vec![];
        let _router = Router::new(buckets);

        // Router should be created successfully even with empty bucket list
        // (The fact that we reach this assertion means router was created successfully)
    }
}
