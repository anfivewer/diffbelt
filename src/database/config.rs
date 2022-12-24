pub struct DatabaseConfig {
    pub query_pack_limit: usize,
    pub query_pack_records_limit: usize,
    /**
     *  This constant used as follows:
     *    - we try to load in memory this number of changed collection keys
     *    - if we are loaded at least one generation, then process them from memory
     *      (and actual `to_generation_id` will be generation on which we are accumulated enough keys)
     *    - if first generation has more keys than this constant says, then we are working in
     *      "iterator over db keys" mode
     *
     *  Later we'll should tune our puts to have at most key updates count as they can in adequate time,
     *  and then merge changed keys from N generations to increase possible number of items in the
     *  single diff. And also save iterated small-size generations to fictive range-generations.
     */
    pub diff_changes_limit: usize,
    pub diff_pack_limit: usize,
    pub diff_pack_records_limit: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            query_pack_limit: 200,
            query_pack_records_limit: 5000,
            diff_changes_limit: 20000,
            diff_pack_limit: 200,
            diff_pack_records_limit: 5000,
        }
    }
}
