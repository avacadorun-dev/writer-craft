//! Disposable ProseMirror-Rust integration evidence; not a production dependency.

/// Convert a Unicode-scalar boundary into a JavaScript/ProseMirror UTF-16 offset.
pub fn scalar_to_utf16(text: &str, scalar: usize) -> Option<usize> {
    if scalar > text.chars().count() {
        return None;
    }
    Some(text.chars().take(scalar).map(char::len_utf16).sum())
}

/// Reject offsets inside a surrogate pair instead of silently moving a quote boundary.
pub fn utf16_to_scalar(text: &str, units: usize) -> Option<usize> {
    let mut consumed = 0;
    for (scalar, ch) in text.chars().enumerate() {
        if consumed == units {
            return Some(scalar);
        }
        consumed += ch.len_utf16();
        if consumed > units {
            return None;
        }
    }
    (consumed == units).then(|| text.chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use prosemirror::{
        dynamic::{DynamicSchema, types::Dyn},
        editor::Editor,
        model::Node,
        transform::{Mappable, Step},
    };
    use serde_json::{Value, json};

    fn schema() -> Value {
        json!({"nodes": {
            "doc": {"content":"block+", "attrs":{"schemaVersion":{"default":4}}},
            "paragraph": {"content":"inline*","group":"block","attrs":{"blockId":{"default":null},"sourceRefs":{"default":[]},"vendor":{"default":null}}},
            "heading": {"content":"inline*","group":"block","attrs":{"level":{"default":1},"blockId":{"default":null},"sourceRefs":{"default":[]}}},
            "text":{"group":"inline"}
        }, "marks":{"strong":{},"em":{}}})
    }

    fn document() -> Value {
        json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
            {"type":"heading","attrs":{"level":2,"blockId":"h-1","sourceRefs":[]},"content":[{"type":"text","text":"Глава первая"}]},
            {"type":"paragraph","attrs":{"blockId":"p-1","sourceRefs":["stt:recording-1:1200-2500"],"vendor":{"unknown":{"keep":[1,"два",true]}}},"content":[
                {"type":"text","text":"Автор 😀 и ё ","marks":[{"type":"strong"}]},
                {"type":"text","text":"👩‍💻 пишет."}
            ]}
        ]})
    }

    fn plain(text: &str) -> Value {
        json!({"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"paragraph","attrs":{"blockId":"p-anchor","sourceRefs":["stt:1"],"vendor":null},"content":[{"type":"text","text":text}]}]})
    }

    #[test]
    fn declared_block_identity_provenance_marks_and_vendor_attributes_roundtrip() {
        let schema = DynamicSchema::from_json(&schema()).expect("schema");
        let input = document();
        let output = schema.with_types(|| {
            let doc = schema.node_from_json(&input).expect("document");
            doc.check().expect("valid schema");
            serde_json::to_value(doc).expect("serialize")
        });
        assert_eq!(output, input);
    }

    #[test]
    fn typed_text_node_sizes_count_utf16_instead_of_scalars_or_bytes() {
        let schema = DynamicSchema::from_json(&schema()).expect("schema");
        let size = schema.with_types(|| schema.text("А😀Б").node_size());
        assert_eq!(size, 4);
    }

    #[test]
    fn every_scalar_boundary_roundtrips_through_utf16_with_combining_and_zwj_text() {
        let text = "Ёж 😀 ё 👩‍💻";
        for scalar in 0..=text.chars().count() {
            let units = scalar_to_utf16(text, scalar).expect("scalar boundary");
            assert_eq!(utf16_to_scalar(text, units), Some(scalar));
        }
    }

    #[test]
    fn utf16_conversion_refuses_the_middle_of_an_emoji_surrogate_pair() {
        assert_eq!(utf16_to_scalar("А😀Б", 2), None);
    }

    #[test]
    fn out_of_bounds_scalar_offsets_are_rejected() {
        assert_eq!(scalar_to_utf16("А😀Б", 4), None);
    }

    #[test]
    fn real_insertion_preserves_attrs_and_maps_the_original_quote() {
        let schema = DynamicSchema::from_json(&schema()).expect("schema");
        let before = "Привет 😀 мир";
        let outcome=schema.with_types(|| {
            let doc=schema.node_from_json(&plain(before)).expect("document");
            let step: Step<Dyn>=serde_json::from_value(json!({"stepType":"replace","from":1,"to":1,"slice":{"content":[{"type":"text","text":"Очень "}]}})).expect("step");
            let result=step.apply(&doc).expect("apply");
            result.check().expect("valid after insert");
            let map=step.get_map();
            let anchor_from=1+scalar_to_utf16(before,7).expect("start");
            let anchor_to=1+scalar_to_utf16(before,8).expect("end");
            let mapped_from=map.map(anchor_from,1)-1;
            let mapped_to=map.map(anchor_to,-1)-1;
            let after=result.text_content();
            let scalar_from=utf16_to_scalar(&after,mapped_from).expect("mapped start");
            let scalar_to=utf16_to_scalar(&after,mapped_to).expect("mapped end");
            let quote:String=after.chars().skip(scalar_from).take(scalar_to-scalar_from).collect();
            (quote, result.attrs_json(),result.child(0).expect("paragraph").attrs_json())
        });
        assert_eq!(
            outcome,
            (
                "😀".to_owned(),
                json!({"schemaVersion":4}),
                json!({"blockId":"p-anchor","sourceRefs":["stt:1"],"vendor":null})
            )
        );
    }

    #[test]
    fn deleting_the_entire_quoted_emoji_flags_its_anchor_as_deleted() {
        let schema = DynamicSchema::from_json(&schema()).expect("schema");
        let deleted = schema.with_types(|| {
            let step: Step<Dyn> = serde_json::from_value(
                json!({"stepType":"replace","from":2,"to":4,"slice":{"content":[]}}),
            )
            .expect("delete emoji step");
            let doc = schema.node_from_json(&plain("А😀Б")).expect("doc");
            let after = step.apply(&doc).expect("delete");
            assert_eq!(after.text_content(), "АБ");
            let from = step.get_map().map_result(2, 1);
            let to = step.get_map().map_result(4, -1);
            from.deleted() && to.deleted() && from.pos == to.pos
        });
        assert!(deleted);
    }

    #[test]
    fn inversion_restores_text_marks_and_provenance_after_real_edit() {
        let schema = DynamicSchema::from_json(&schema()).expect("schema");
        let input = document();
        let result=schema.with_types(|| {
            let doc=schema.node_from_json(&input).expect("doc");
            let step:Step<Dyn>=serde_json::from_value(json!({"stepType":"replace","from":1,"to":1,"slice":{"content":[{"type":"text","text":"Новая "}]}})).expect("step");
            let inverse=step.invert(&doc);
            let after=step.apply(&doc).expect("insert");
            let restored=inverse.apply(&after).expect("undo");
            serde_json::to_value(restored).expect("serialize")
        });
        assert_eq!(result, input);
    }

    #[test]
    fn stateful_editor_rolls_back_an_invalid_step_batch() {
        let original = plain("А😀Б");
        let mut editor = Editor::new(&schema().to_string(), &original.to_string()).expect("editor");
        let batch = json!([
            {"stepType":"replace","from":1,"to":1,"slice":{"content":[{"type":"text","text":"Новое "}]}},
            {"stepType":"replace","from":10000,"to":10001,"slice":{"content":[]}}
        ]);
        assert!(
            !editor
                .apply_steps_json(&batch.to_string())
                .expect("batch parsed")
        );
        assert_eq!(
            serde_json::from_str::<Value>(&editor.doc_json(false).expect("json")).expect("parse"),
            original
        );
        assert_eq!(editor.version(), 0);
    }

    #[test]
    fn position_mapping_alone_does_not_prove_a_quote_is_still_exact() {
        let schema = DynamicSchema::from_json(&schema()).expect("schema");
        let outcome = schema.with_types(|| {
            let doc = schema
                .node_from_json(&plain("Привет мир"))
                .expect("document");
            let step: Step<Dyn> = serde_json::from_value(json!({
                "stepType":"replace", "from":3,"to":4,
                "slice":{"content":[{"type":"text","text":"ы"}]}
            }))
            .expect("step");
            let after = step.apply(&doc).expect("apply");
            let from = step.get_map().map_result(1, 1);
            let to = step.get_map().map_result(7, -1);
            let quote: String = after.text_content().chars().take(6).collect();
            (!from.deleted() && !to.deleted(), quote)
        });
        assert_eq!(outcome, (true, "Прывет".to_owned()));
    }

    #[test]
    fn unknown_node_type_is_not_rejected_during_json_parse_alone() {
        let schema = DynamicSchema::from_json(&schema()).expect("schema");
        let unknown = json!({"type":"doc","content":[{"type":"futureRichBlock","attrs":{"opaque":{"keep":true}},"content":[]}]});
        let parsed = schema.node_from_json(&unknown);
        assert!(
            parsed.is_ok(),
            "record actual parser behavior; adapter must prevalidate names"
        );
    }

    #[test]
    fn unknown_top_level_node_fields_are_not_a_lossless_extension_channel() {
        let schema = DynamicSchema::from_json(&schema()).expect("schema");
        let mut input = plain("Текст");
        input["vendorTopLevel"] = json!({"mustSurvive":true});
        let output = schema.with_types(|| {
            serde_json::to_value(schema.node_from_json(&input).expect("doc")).expect("serialize")
        });
        assert!(output.get("vendorTopLevel").is_none());
    }
}
