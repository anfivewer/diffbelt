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

    /**
     * Note that when you are starting cursor there is 1 public cursor id (A)
     * then when you are fetching this cursor, there is 2 public cursors id (A and B)
     * cursor A will be dropped after requesting B, but there will be always 2 cursors
     * (current and next) until last cursor will be fetched.
     * Last cursor always contains empty items list and stays present infinite long
     * until this limit will not be reached
     */
    pub max_cursors_per_collection: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            query_pack_limit: 200,
            query_pack_records_limit: 5000,
            diff_changes_limit: 20000,
            diff_pack_limit: 200,
            diff_pack_records_limit: 5000,
            max_cursors_per_collection: 100,
        }
    }
}
