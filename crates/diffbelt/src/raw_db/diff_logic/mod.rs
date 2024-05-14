use crate::collection::util::record_key::{
    OwnedRecordKey, ParsedRecordKey, ParsedRecordKeyRanges, RecordKey,
};
use crate::common::{CollectionKey, GenerationId, IsByteArray, OwnedCollectionKey};
use crate::raw_db::diff_logic::error::DiffLogicError;
use crate::raw_db::diff_logic::input::DiffLogicInput;
use diffbelt_util::debug_print::debug_print;
use std::fmt::format;
use std::mem;

pub mod error;
pub mod input;

pub struct EmitItemAction<'a> {
    pub key: CollectionKey<'a>,
    pub from_key: Option<RecordKey<'a>>,
    pub to_key: Option<RecordKey<'a>>,
    pub next_action: DiffLogicSubAction<'a>,
}

const WITH_DEBUG_PRINTS: bool = false;

pub enum DiffLogicSubAction<'a> {
    SetCursorAndGetNextCursorKeyAndNextChangedKey(RecordKey<'a>),
    GetNextCursorKeyAndNextChangedKey,
    Finish(Option<ResumeDiffLogic>),
}

pub enum DiffLogicAction<'a> {
    GetNextChangedKey,
    SetCursorAndGetNext(RecordKey<'a>),
    GetNextCursorKey,
    EmitItem(EmitItemAction<'a>),
    Sub(DiffLogicSubAction<'a>),
}

type ParsedRecordKeyHolder = (ParsedRecordKeyRanges, Vec<u8>);
type ParsedRecordKeyHolderOptional = (Option<ParsedRecordKeyRanges>, Vec<u8>);

type CollectionKeyHolder = Vec<u8>;
type CollectionKeyHolderOptional = (bool, Vec<u8>);

type HandlerResult<'a> = Result<DiffLogicAction<'a>, DiffLogicError>;

pub struct DiffLogic<'a> {
    from_generation_id: GenerationId<'a>,
    to_generation_id: GenerationId<'a>,
    is_required_to_stop: bool,
    items_limit: usize,
    handler:
        for<'b> fn(&'b mut Self, DiffLogicInput<'_>) -> Result<DiffLogicAction<'b>, DiffLogicError>,
    changed_key_holder: CollectionKeyHolder,
    next_changed_key_holder: CollectionKeyHolderOptional,
    current_record_key_holder: ParsedRecordKeyHolder,
    first_key_holder: ParsedRecordKeyHolderOptional,
    last_key_holder: ParsedRecordKeyHolderOptional,

    temp_key_holder: ParsedRecordKeyHolder,
}

#[derive(Debug)]
pub struct ResumeDiffLogic {
    pub action: DiffLogicResumeAction,
    pub changed_key: OwnedCollectionKey,
    pub next_changed_key: Option<OwnedCollectionKey>,
    pub current_record_key: OwnedRecordKey,
    pub first_key: Option<OwnedRecordKey>,
    pub last_key: Option<OwnedRecordKey>,
}

#[derive(Debug)]
pub enum DiffLogicResumeAction {
    SetCursorAndGetNextCursorKeyAndNextChangedKey(OwnedRecordKey),
    GetNextCursorKeyAndNextChangedKey,
    GetNextCursorKey,
}

pub struct NewDiffLogic<'a> {
    pub from_generation_id: Option<GenerationId<'a>>,
    pub to_generation_id: GenerationId<'a>,
    pub items_limit: usize,
}

macro_rules! make_handler {
    ($this:ident, $input:ident, $body:block) => {{
        fn handler<'a, 'b>(
            $this: &'b mut DiffLogic<'a>,
            $input: DiffLogicInput<'_>,
        ) -> Result<DiffLogicAction<'b>, DiffLogicError> {
            $body
        }

        handler
    }};
}

impl<'a> NewDiffLogic<'a> {
    pub fn new(self) -> DiffLogic<'a> {
        DiffLogic {
            from_generation_id: self.from_generation_id.unwrap_or(GenerationId::empty()),
            to_generation_id: self.to_generation_id,
            is_required_to_stop: false,
            items_limit: self.items_limit,
            handler: make_handler!(this, input, {
                if !input.is_init() {
                    return Err(DiffLogicError::Unspecified(
                        "Expected Init as first action".to_string(),
                    ));
                }

                this.init()
            }),
            changed_key_holder: Vec::new(),
            next_changed_key_holder: (false, Vec::new()),
            current_record_key_holder: (ParsedRecordKeyRanges::default(), Vec::new()),
            first_key_holder: (None, Vec::new()),
            last_key_holder: (None, Vec::new()),
            temp_key_holder: (ParsedRecordKeyRanges::default(), Vec::new()),
        }
    }

    pub fn resume(self, resume: &ResumeDiffLogic) -> DiffLogic<'a> {
        let ResumeDiffLogic {
            action,
            changed_key,
            next_changed_key,
            current_record_key,
            first_key,
            last_key,
        } = resume;

        let current_record_key = current_record_key.as_ref().parse();
        let first_key = first_key.as_ref().map(|x| x.as_ref().parse());
        let last_key = last_key.as_ref().map(|x| x.as_ref().parse());

        let mut changed_key_holder = Vec::new();
        let mut next_changed_key_holder = (false, Vec::new());
        let mut current_record_key_holder = (ParsedRecordKeyRanges::default(), Vec::new());
        let mut first_key_holder = (None, Vec::new());
        let mut last_key_holder = (None, Vec::new());
        let mut temp_key_holder = (ParsedRecordKeyRanges::default(), Vec::new());

        write_collection_key_to_holder(&mut changed_key_holder, changed_key.as_ref());
        write_collection_key_to_holder_optional(
            &mut next_changed_key_holder,
            next_changed_key.as_ref().map(|x| x.as_ref()),
        );
        write_key_to_holder(&mut current_record_key_holder, &current_record_key);
        write_key_to_holder_optional(&mut first_key_holder, first_key.as_ref());
        write_key_to_holder_optional(&mut last_key_holder, last_key.as_ref());

        let handler = match action {
            DiffLogicResumeAction::SetCursorAndGetNextCursorKeyAndNextChangedKey(key) => {
                let key = key.as_ref().parse();
                write_key_to_holder(&mut temp_key_holder, &key);

                make_handler!(this, input, {
                    if !input.is_init() {
                        return Err(DiffLogicError::Unspecified(
                            "Expected Init as first action".to_string(),
                        ));
                    }

                    let key = parsed_key_from_holder(&this.temp_key_holder);

                    this.handler = make_handler!(this, input, {
                        let (cursor_key, changed_key) =
                            input.into_next_cursor_and_changed_key().map_err(|_| {
                                DiffLogicError::Unspecified(
                                    "Expected NextCursorAndChangedKey".to_string(),
                                )
                            })?;
                        this.on_got_next_key_and_changed_key(cursor_key, changed_key)
                    });

                    Ok(DiffLogicAction::Sub(
                        DiffLogicSubAction::SetCursorAndGetNextCursorKeyAndNextChangedKey(
                            key.as_record_key(),
                        ),
                    ))
                })
            }
            DiffLogicResumeAction::GetNextCursorKeyAndNextChangedKey => {
                make_handler!(this, input, {
                    if !input.is_init() {
                        return Err(DiffLogicError::Unspecified(
                            "Expected Init as first action".to_string(),
                        ));
                    }

                    this.handler = make_handler!(this, input, {
                        let (cursor_key, changed_key) =
                            input.into_next_cursor_and_changed_key().map_err(|_| {
                                DiffLogicError::Unspecified(
                                    "Expected NextCursorAndChangedKey".to_string(),
                                )
                            })?;
                        this.on_got_next_key_and_changed_key_with_existing_current(
                            cursor_key,
                            changed_key,
                        )
                    });

                    Ok(DiffLogicAction::Sub(
                        DiffLogicSubAction::GetNextCursorKeyAndNextChangedKey,
                    ))
                })
            }
            DiffLogicResumeAction::GetNextCursorKey => {
                make_handler!(this, input, {
                    if !input.is_init() {
                        return Err(DiffLogicError::Unspecified(
                            "Expected Init as first action".to_string(),
                        ));
                    }

                    this.handler = make_handler!(this, input, {
                        let input = input.into_next_cursor_key().map_err(|_| {
                            DiffLogicError::Unspecified("Expected NextCursorKey".to_string())
                        })?;
                        this.on_got_next_cursor(input)
                    });

                    Ok(DiffLogicAction::GetNextCursorKey)
                })
            }
        };

        DiffLogic {
            from_generation_id: self.from_generation_id.unwrap_or(GenerationId::empty()),
            to_generation_id: self.to_generation_id,
            is_required_to_stop: false,
            items_limit: self.items_limit,
            handler,
            changed_key_holder,
            next_changed_key_holder,
            current_record_key_holder,
            first_key_holder,
            last_key_holder,
            temp_key_holder,
        }
    }
}

impl<'a> DiffLogic<'a> {
    pub fn run<'b>(&'b mut self, input: DiffLogicInput<'_>) -> HandlerResult<'b> {
        (self.handler)(self, input)
    }

    pub fn require_to_stop(&mut self) {
        self.is_required_to_stop = true;
    }

    fn init(&mut self) -> Result<DiffLogicAction<'a>, DiffLogicError> {
        self.handler = make_handler!(this, input, {
            let input = input
                .into_next_changed_key()
                .map_err(|_| DiffLogicError::Unspecified("Expected NextChangedKey".to_string()))?;
            this.on_got_first_changed_key(input)
        });
        Ok(DiffLogicAction::GetNextChangedKey)
    }

    fn on_got_first_changed_key(
        &mut self,
        changed_key: Option<CollectionKey<'_>>,
    ) -> HandlerResult<'_> {
        if WITH_DEBUG_PRINTS {
            debug_print(format!("on_got_first_changed_key: {changed_key:?}").as_str());
        }

        let Some(changed_key) = changed_key else {
            self.handler = make_handler!(_this, _input, { Err(DiffLogicError::AlreadyFinished) });
            return Ok(DiffLogicAction::Sub(DiffLogicSubAction::Finish(None)));
        };

        write_collection_key_to_holder(&mut self.changed_key_holder, changed_key);

        self.handler = make_handler!(this, input, {
            let input = input
                .into_next_changed_key()
                .map_err(|_| DiffLogicError::Unspecified("Expected NextChangedKey".to_string()))?;
            this.on_got_next_changed_key(input)
        });
        Ok(DiffLogicAction::GetNextChangedKey)
    }

    fn on_got_next_changed_key(
        &mut self,
        next_changed_key: Option<CollectionKey<'_>>,
    ) -> HandlerResult<'_> {
        if WITH_DEBUG_PRINTS {
            debug_print(format!("on_got_next_changed_key: {next_changed_key:?}").as_str());
        }

        write_collection_key_to_holder_optional(
            &mut self.next_changed_key_holder,
            next_changed_key,
        );

        let changed_key = collection_key_from_holder(&self.changed_key_holder);

        // Start from empty generation to find first value
        let key_with_no_generation =
            make_no_generation_record_key(&mut self.temp_key_holder, changed_key)?;

        self.handler = make_handler!(this, input, {
            let input = input
                .into_next_cursor_key()
                .map_err(|_| DiffLogicError::Unspecified("Expected NextCursorKey".to_string()))?;
            this.on_got_next_after_cursor_set(input)
        });
        Ok(DiffLogicAction::SetCursorAndGetNext(key_with_no_generation))
    }

    fn on_got_next_after_cursor_set(
        &mut self,
        key: Option<ParsedRecordKey<'_>>,
    ) -> Result<DiffLogicAction<'_>, DiffLogicError> {
        if WITH_DEBUG_PRINTS {
            debug_print(format!("on_got_next_after_cursor_set: {key:?}").as_str());
        }

        let Some(key) = key else {
            self.handler = make_handler!(_this, _input, { Err(DiffLogicError::AlreadyErrored) });
            return Err(DiffLogicError::ChangedKeyNotFound(
                "on_got_next_after_cursor_set",
            ));
        };

        let changed_key = collection_key_from_holder(&self.changed_key_holder);

        if key.collection_key() != changed_key {
            self.handler = make_handler!(_this, _input, { Err(DiffLogicError::AlreadyErrored) });
            return Err(DiffLogicError::Unspecified(
                "First iterator key is not changed one".to_string(),
            ));
        }

        write_key_to_holder(&mut self.current_record_key_holder, &key);

        self.handler = make_handler!(this, input, {
            let input = input
                .into_next_cursor_key()
                .map_err(|_| DiffLogicError::Unspecified("Expected NextCursorKey".to_string()))?;
            this.on_got_next_cursor(input)
        });
        Ok(DiffLogicAction::GetNextCursorKey)
    }

    fn on_got_next_cursor(
        &mut self,
        key: Option<ParsedRecordKey<'_>>,
    ) -> Result<DiffLogicAction<'_>, DiffLogicError> {
        if WITH_DEBUG_PRINTS {
            debug_print(format!("on_got_next_cursor: {key:?}").as_str());
        }
        self.handle_keys_pair(key)
    }

    fn handle_keys_pair(
        &mut self,
        next_key: Option<ParsedRecordKey<'_>>,
    ) -> Result<DiffLogicAction<'_>, DiffLogicError> {
        let current_key = parsed_key_from_holder(&self.current_record_key_holder);
        let current_collection_key = current_key.collection_key();

        if WITH_DEBUG_PRINTS {
            debug_print(format!("handle_keys_pair: {current_key:?} {next_key:?}").as_str());
        }

        // If this is last key, or collection key differs, or next generation is greater than we
        // need, then this is the end for this key
        let is_last_key = next_key.as_ref().map_or(true, |key| {
            key.collection_key() != current_collection_key
                || key.generation_id() > self.to_generation_id
        });

        let mut is_next_key_is_new_first = false;
        let mut is_next_key_is_new_last = false;

        let current_key_is_phantom = current_key.phantom_id().is_some();

        // View for next key, maybe no need to copy current key to first/last yet
        // No need to do it if current key is phantom, it will not be written
        if !is_last_key && !current_key_is_phantom {
            if let Some(next_key) = next_key.as_ref() {
                let generation_id = next_key.generation_id();

                if generation_id <= self.from_generation_id {
                    is_next_key_is_new_first = true;
                }
                if generation_id <= self.to_generation_id {
                    is_next_key_is_new_last = true;
                }
            }
        }

        // Write current key to first/last holders
        if !current_key_is_phantom {
            let generation_id = current_key.generation_id();

            if !is_next_key_is_new_first && generation_id <= self.from_generation_id {
                write_key_to_holder_optional(&mut self.first_key_holder, Some(&current_key));
            }
            if !is_next_key_is_new_last && generation_id <= self.to_generation_id {
                write_key_to_holder_optional(&mut self.last_key_holder, Some(&current_key));
            }
        }

        if !is_last_key {
            // Store next key as current and continue
            write_key_to_holder(
                &mut self.current_record_key_holder,
                next_key.as_ref().expect("already checked"),
            );

            if self.is_required_to_stop {
                return Ok(DiffLogicAction::Sub(DiffLogicSubAction::Finish(Some(
                    Self::make_resume(
                        DiffLogicResumeAction::GetNextCursorKey,
                        &self.changed_key_holder,
                        &self.next_changed_key_holder,
                        &self.current_record_key_holder,
                        &self.first_key_holder,
                        &self.last_key_holder,
                    ),
                ))));
            }

            self.handler = make_handler!(this, input, {
                let input = input.into_next_cursor_key().map_err(|_| {
                    DiffLogicError::Unspecified("Expected NextCursorKey".to_string())
                })?;
                this.on_got_next_cursor(input)
            });
            return Ok(DiffLogicAction::GetNextCursorKey);
        }

        // This is last key. In first/current holders we now should contain keys before
        // from_generation_id and to_generation_id.

        let first_key = parsed_key_from_holder_optional(&self.first_key_holder);
        let last_key = parsed_key_from_holder_optional(&self.last_key_holder);

        if first_key == last_key {
            // It is possible only if from_generation_id == to_generation_id, but it should be
            // checked before diff
            self.handler = make_handler!(_this, _input, { Err(DiffLogicError::AlreadyErrored) });
            return Err(DiffLogicError::ChangeNotFound);
        }

        let next_changed_key = move_collection_key_from_optional_to_holder(
            &mut self.next_changed_key_holder,
            &mut self.changed_key_holder,
        );

        let next_action = 'block: {
            let Some(next_changed_key) = next_changed_key else {
                // No more changed keys to see
                self.handler =
                    make_handler!(_this, _input, { Err(DiffLogicError::AlreadyFinished) });
                break 'block DiffLogicSubAction::Finish(None);
            };

            if let Some(next_key) = next_key.as_ref() {
                if next_key.collection_key() == next_changed_key {
                    // If next changed key is following current key, no need to set cursor to it,
                    // just continue traversing
                    write_key_to_holder(&mut self.current_record_key_holder, &*next_key);

                    self.handler = make_handler!(this, input, {
                        let (cursor_key, changed_key) =
                            input.into_next_cursor_and_changed_key().map_err(|_| {
                                DiffLogicError::Unspecified(
                                    "Expected NextCursorAndChangedKey".to_string(),
                                )
                            })?;
                        this.on_got_next_key_and_changed_key_with_existing_current(
                            cursor_key,
                            changed_key,
                        )
                    });
                    break 'block DiffLogicSubAction::GetNextCursorKeyAndNextChangedKey;
                }
            }

            self.handler = make_handler!(this, input, {
                let (cursor_key, changed_key) =
                    input.into_next_cursor_and_changed_key().map_err(|_| {
                        DiffLogicError::Unspecified("Expected NextCursorAndChangedKey".to_string())
                    })?;
                this.on_got_next_key_and_changed_key(cursor_key, changed_key)
            });

            let key_with_no_generation =
                make_no_generation_record_key(&mut self.temp_key_holder, next_changed_key)?;

            DiffLogicSubAction::SetCursorAndGetNextCursorKeyAndNextChangedKey(
                key_with_no_generation,
            )
        };

        self.items_limit -= 1;

        if WITH_DEBUG_PRINTS {
            debug_print(format!("limit {}", self.items_limit).as_str());
        }

        let next_action = 'block: {
            if self.items_limit == 0 || self.is_required_to_stop {
                let action = match next_action {
                    DiffLogicSubAction::SetCursorAndGetNextCursorKeyAndNextChangedKey(key) => {
                        DiffLogicResumeAction::SetCursorAndGetNextCursorKeyAndNextChangedKey(
                            key.to_owned(),
                        )
                    }
                    DiffLogicSubAction::GetNextCursorKeyAndNextChangedKey => {
                        DiffLogicResumeAction::GetNextCursorKeyAndNextChangedKey
                    }
                    action @ DiffLogicSubAction::Finish(_) => {
                        break 'block action;
                    }
                };

                DiffLogicSubAction::Finish(Some(Self::make_resume(
                    action,
                    &self.changed_key_holder,
                    &self.next_changed_key_holder,
                    &self.current_record_key_holder,
                    &self.first_key_holder,
                    &self.last_key_holder,
                )))
            } else {
                next_action
            }
        };

        let key = first_key.as_ref().or(last_key.as_ref()).ok_or_else(|| {
            DiffLogicError::Unspecified(
                "Both first and last keys are None, but we are checked it".to_string(),
            )
        })?;

        Ok(DiffLogicAction::EmitItem(EmitItemAction {
            key: key.collection_key(),
            from_key: first_key.as_ref().map(|x| x.as_record_key()),
            to_key: last_key.as_ref().map(|x| x.as_record_key()),
            next_action,
        }))
    }

    fn on_got_next_key_and_changed_key_with_existing_current(
        &mut self,
        key: Option<ParsedRecordKey<'_>>,
        next_changed_key: Option<CollectionKey<'_>>,
    ) -> Result<DiffLogicAction<'_>, DiffLogicError> {
        if WITH_DEBUG_PRINTS {
            debug_print(format!("on_got_next_key_and_changed_key_with_existing_current: {key:?}, {next_changed_key:?}").as_str());
        }

        self.first_key_holder.0 = None;
        self.last_key_holder.0 = None;

        write_collection_key_to_holder_optional(
            &mut self.next_changed_key_holder,
            next_changed_key,
        );

        self.handle_keys_pair(key)
    }

    fn on_got_next_key_and_changed_key(
        &mut self,
        key: Option<ParsedRecordKey<'_>>,
        next_changed_key: Option<CollectionKey<'_>>,
    ) -> Result<DiffLogicAction<'_>, DiffLogicError> {
        if WITH_DEBUG_PRINTS {
            debug_print(
                format!("on_got_next_key_and_changed_key: {key:?}, {next_changed_key:?}").as_str(),
            );
        }

        let Some(key) = key else {
            self.handler = make_handler!(_this, _input, { Err(DiffLogicError::AlreadyErrored) });
            return Err(DiffLogicError::ChangedKeyNotFound(
                "on_got_next_key_and_changed_key",
            ));
        };

        self.first_key_holder.0 = None;
        self.last_key_holder.0 = None;

        write_collection_key_to_holder_optional(
            &mut self.next_changed_key_holder,
            next_changed_key,
        );

        write_key_to_holder(&mut self.current_record_key_holder, &key);

        self.handler = make_handler!(this, input, {
            let input = input
                .into_next_cursor_key()
                .map_err(|_| DiffLogicError::Unspecified("Expected NextCursorKey".to_string()))?;
            this.on_got_next_cursor(input)
        });
        Ok(DiffLogicAction::GetNextCursorKey)
    }

    fn make_resume(
        action: DiffLogicResumeAction,
        changed_key_holder: &CollectionKeyHolder,
        next_changed_key_holder: &CollectionKeyHolderOptional,
        current_record_key_holder: &ParsedRecordKeyHolder,
        first_key_holder: &ParsedRecordKeyHolderOptional,
        last_key_holder: &ParsedRecordKeyHolderOptional,
    ) -> ResumeDiffLogic {
        ResumeDiffLogic {
            action,
            changed_key: collection_key_from_holder(changed_key_holder).to_owned(),
            next_changed_key: collection_key_from_holder_optional(next_changed_key_holder)
                .map(|x| x.to_owned()),
            current_record_key: parsed_key_from_holder(current_record_key_holder)
                .as_record_key()
                .to_owned(),
            first_key: parsed_key_from_holder_optional(first_key_holder)
                .map(|x| x.as_record_key().to_owned()),
            last_key: parsed_key_from_holder_optional(last_key_holder)
                .map(|x| x.as_record_key().to_owned()),
        }
    }
}

fn parsed_key_from_holder(holder: &ParsedRecordKeyHolder) -> ParsedRecordKey<'_> {
    let (parts, holder) = holder;
    ParsedRecordKey::new_unchecked(&holder, parts.clone())
}

fn parsed_key_from_holder_optional(
    holder: &ParsedRecordKeyHolderOptional,
) -> Option<ParsedRecordKey<'_>> {
    let (parts, holder) = holder;

    let Some(parts) = parts else {
        return None;
    };

    Some(ParsedRecordKey::new_unchecked(&holder, parts.clone()))
}

fn record_key_from_holder(holder: &ParsedRecordKeyHolder) -> RecordKey<'_> {
    let (_, holder) = holder;
    RecordKey::new_unchecked(&holder)
}

fn write_key_to_holder(holder: &mut ParsedRecordKeyHolder, key: &ParsedRecordKey<'_>) {
    let (parts, holder) = holder;

    holder.clear();
    holder.extend_from_slice(key.bytes());
    *parts = key.ranges().clone();
}

fn write_key_to_holder_optional(
    holder: &mut ParsedRecordKeyHolderOptional,
    key: Option<&ParsedRecordKey<'_>>,
) {
    let (parts, holder) = holder;

    let Some(key) = key else {
        *parts = None;
        return;
    };

    holder.clear();
    holder.extend_from_slice(key.bytes());
    *parts = Some(key.ranges().clone());
}

fn move_key_from_optional_to_holder<'a>(
    from_holder: &'a mut ParsedRecordKeyHolderOptional,
    to_holder: &'a mut ParsedRecordKeyHolder,
) -> Option<ParsedRecordKey<'a>> {
    let (from_parts, from_holder) = from_holder;

    let Some(from_parts) = from_parts.take() else {
        return None;
    };

    let (to_parts, to_holder) = to_holder;

    *to_parts = from_parts;
    mem::swap(from_holder, to_holder);

    Some(ParsedRecordKey::new_unchecked(
        &*to_holder,
        to_parts.clone(),
    ))
}

fn collection_key_from_holder(holder: &CollectionKeyHolder) -> CollectionKey<'_> {
    CollectionKey::new_unchecked(holder.as_slice())
}

fn collection_key_from_holder_optional(
    holder: &CollectionKeyHolderOptional,
) -> Option<CollectionKey<'_>> {
    let (exists, holder) = holder;

    if !*exists {
        return None;
    }

    Some(CollectionKey::new_unchecked(&holder))
}

fn write_collection_key_to_holder(holder: &mut CollectionKeyHolder, key: CollectionKey<'_>) {
    holder.clear();
    holder.extend_from_slice(key.get_byte_array());
}

fn write_collection_key_to_holder_optional(
    holder: &mut CollectionKeyHolderOptional,
    key: Option<CollectionKey<'_>>,
) {
    let (exists, holder) = holder;

    let Some(key) = key else {
        *exists = false;
        return;
    };

    holder.clear();
    holder.extend_from_slice(key.get_byte_array());
    *exists = true;
}

fn move_collection_key_from_optional_to_holder<'a>(
    from_holder: &'a mut CollectionKeyHolderOptional,
    to_holder: &'a mut CollectionKeyHolder,
) -> Option<CollectionKey<'a>> {
    let (from_exists, from_holder) = from_holder;

    if !*from_exists {
        return None;
    };

    mem::swap(from_holder, to_holder);

    Some(CollectionKey::new_unchecked(to_holder.as_slice()))
}

fn make_no_generation_record_key<'a>(
    holder: &'a mut ParsedRecordKeyHolder,
    changed_key: CollectionKey<'_>,
) -> Result<RecordKey<'a>, DiffLogicError> {
    let key_with_no_generation =
        ParsedRecordKey::new_on_vec(&mut holder.1, changed_key, GenerationId::empty(), None)
            .map_err(|()| DiffLogicError::InvalidRecordKey)?;

    holder.0 = key_with_no_generation.ranges().clone();

    Ok(record_key_from_holder(&*holder))
}
