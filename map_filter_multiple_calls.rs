struct Responses {
  from_collection: Option<EncodedGenerationIdJsonDataBytesCow>,
  to_collection: Option<EncodedGenerationIdJsonDataBytesCow>,
}

impl Responses {
  fn finalize(&self, this: &mut MapFilterTransform) -> Result<ActionInputHandlerResult, TransformError> {
      let Some(from_collection_generation_id) = &self.from_collection else {
          return Ok(ActionInputHandlerResult::Consumed);
      };
      let Some(to_collection_generation_id) = &self.to_collection else {
          return Ok(ActionInputHandlerResult::Consumed);
      };

      let from_collection_generation_id = from_collection_generation_id.as_bytes();
      let to_collection_generation_id = to_collection_generation_id.as_bytes();

      if to_collection_generation_id >= from_collection_generation_id {
          return Ok(ActionInputHandlerResult::Finish);
      }

      //

      todo!()
  }
}

let responses: Rc<RefCell<Responses>> = Wrap::wrap(Responses {
  from_collection: None,
  to_collection: None,
});

fn get_generation_id(input: InputType) -> Result<Option<EncodedGenerationIdJsonData>, TransformError> {
  let InputType::DiffbeltCall(call) = input else {
      return Err(TransformError::Unspecified(
          "Not a diffbelt call response".to_string(),
      ));
  };

  let DiffbeltCallInput { body } = call;

  let DiffbeltResponseBody::GetCollection(response) = body else {
      return Err(TransformError::Unspecified(
          "Not a get_collection call response".to_string(),
      ));
  };

  let GetCollectionResponseJsonData { generation_id, .. } = response;

  Ok(generation_id)
}

self.push_action(
  &mut actions,
  ActionType::DiffbeltCall(DiffbeltCallAction {
      method: Method::Post,
      path: Cow::Borrowed("/raw/get_collection"),
      body: DiffbeltRequestBody::GetCollection(GetCollectionRequestJsonData {
          collection_id: self.from_collection_name.to_string(),
          with_generation_id: Some(true),
          with_next_generation_id: None,
      }),
  }),
  {
      let responses = responses.clone();
      Box::new(move |this, input| {
          let generation_id = get_generation_id(input)?;

          let Some(generation_id) = generation_id else {
              // initial collection has no generation_id, nothing to process
              return Ok(ActionInputHandlerResult::Finish);
          };

          let bytes = generation_id.into_bytes()?;

          let mut responses = responses.borrow_mut();
          let responses = responses.deref_mut();

          let _ = responses.from_collection.insert(bytes);

          responses.finalize(this)
      })
  },
);

self.push_action(
  &mut actions,
  ActionType::DiffbeltCall(DiffbeltCallAction {
      method: Method::Post,
      path: Cow::Borrowed("/raw/get_collection"),
      body: DiffbeltRequestBody::GetCollection(GetCollectionRequestJsonData {
          collection_id: self.to_collection_name.to_string(),
          with_generation_id: Some(true),
          with_next_generation_id: None,
      }),
  }),
  Box::new(move |this, input| {
      let generation_id = get_generation_id(input)?;

      let Some(generation_id) = generation_id else {
          return Err(TransformError::Unspecified(
              "Target collection has no generation_id".to_string(),
          ));
      };

      let bytes = generation_id.into_bytes()?;

      let mut responses = responses.borrow_mut();
      let responses = responses.deref_mut();

      let _ = responses.to_collection.insert(bytes);

      responses.finalize(this)
  }),
);
