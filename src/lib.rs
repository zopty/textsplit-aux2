mod parser;
use parser::{
    alignment::{HDir, VDir, parse_alignment},
    markup::parse_markup,
};

mod entry;
use entry::TEXT_ALIAS_TEMPLATE;

use aviutl2::{
    AnyResult, anyhow,
    generic::{EditSection, GenericPlugin},
    log,
};

#[aviutl2::plugin(GenericPlugin)]
struct TextSplit {}

impl GenericPlugin for TextSplit {
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        aviutl2::logger::LogBuilder::new()
            .filter_level(log::LevelFilter::Info)
            .try_init()?;
        Ok(TextSplit {})
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        registry.set_plugin_information(&format!("Text Split"));
        registry.register_menus::<TextSplit>();
    }
}

#[aviutl2::generic::menus]
impl TextSplit {
    #[object(name = "テキストを分割")]
    fn split_text(edit_section: &mut aviutl2::generic::EditSection) -> AnyResult<()> {
        let selected_objects = edit_section.get_selected_objects()?;
        let mut creation_infos = Vec::new();
        let mut objects_to_delete = Vec::new();

        // Phase 1: Read all data from objects without mutation.
        for obj_handle in &selected_objects {
            let obj = edit_section.object(obj_handle);

            let text_result = obj.get_effect_item("テキスト", 0, "テキスト");
            let text = match text_result {
                Ok(t) => t,
                Err(_) => {
                    continue;
                }
            };

            let elements = parse_markup(&text)
                .map_err(|e| anyhow::anyhow!("テキストの解析に失敗しました: {}: {}", text, e))?;

            let layer_frame = obj.get_layer_frame()?;
            let _layer = layer_frame.layer;
            let start = layer_frame.start;
            let end = layer_frame.end;

            let _size: f32 = obj.get_effect_item("テキスト", 0, "サイズ")?.parse()?;
            let kern: f32 = obj.get_effect_item("テキスト", 0, "字間")?.parse()?;
            let lnsp: f32 = obj.get_effect_item("テキスト", 0, "行間")?.parse()?;
            let font = obj.get_effect_item("テキスト", 0, "フォント")?;
            let color = obj.get_effect_item("テキスト", 0, "文字色")?;
            let subcolor = obj.get_effect_item("テキスト", 0, "影・縁色")?;
            let style = obj.get_effect_item("テキスト", 0, "文字装飾")?;
            let bold = obj.get_effect_item("テキスト", 0, "B")?;
            let italic = obj.get_effect_item("テキスト", 0, "I")?;

            let _x: f32 = obj
                .get_effect_item("標準描画", 0, "X")
                .unwrap_or("0.0".to_string())
                .parse()?;
            let _y: f32 = obj
                .get_effect_item("標準描画", 0, "Y")
                .unwrap_or("0.0".to_string())
                .parse()?;
            let z: f32 = obj
                .get_effect_item("標準描画", 0, "Z")
                .unwrap_or("0.0".to_string())
                .parse()?;
            let alpha = obj.get_effect_item("標準描画", 0, "透明度")?;
            let blend = obj.get_effect_item("標準描画", 0, "合成モード")?;

            let alignment = obj
                .get_effect_item("テキスト", 0, "文字揃え")
                .map(|align| parse_alignment(&align))?;

            let mut w: f32 = 0.0;
            let mut w_temp: f32 = 0.0;
            let mut h: f32 = 0.0;
            let mut h_temp: f32 = 0.0;
            for el in elements.clone() {
                if el.text == "\\n" {
                    w = w.max(w_temp);
                    h += h_temp + lnsp;
                    w_temp = 0.0;
                    h_temp = 0.0;
                    continue;
                }
                let size = el.size.unwrap_or(_size);
                w_temp += size + kern;
                h_temp = h_temp.max(size);
            }

            match alignment.hdir {
                HDir::Left => {
                    w = 0.0;
                }
                HDir::Mid => {
                    w *= 0.5;
                }
                HDir::Right => (),
            }

            match alignment.vdir {
                VDir::Top => {
                    h = 0.0;
                }
                VDir::Center => {
                    h *= 0.5;
                }
                VDir::Bottom => (),
            }

            let mut x = _x - w;
            let mut y = _y - h;

            let mut layer = _layer + 1;

            for el in elements.clone() {
                if el.text == "\\n" {
                    x = _x - w;
                    y += _size + lnsp;
                    continue;
                }
                for c in el.text.chars() {
                    let size = el.size.unwrap_or(_size);
                    let alias = TEXT_ALIAS_TEMPLATE
                        .replace("{start}", &start.to_string())
                        .replace("{end}", &end.to_string())
                        .replace("{size}", &format!("{:.2}", size))
                        .replace("{font}", &el.font.as_ref().unwrap_or(&font))
                        .replace("{color}", &el.color.as_ref().unwrap_or(&color))
                        .replace("{subcolor}", &subcolor.to_string())
                        .replace("{style}", &style.to_string())
                        .replace(
                            "{bold}",
                            if let Some(is_bold) = el.is_bold {
                                if is_bold { "1" } else { "0" }
                            } else {
                                &bold
                            },
                        )
                        .replace(
                            "{italic}",
                            if let Some(is_italic) = el.is_italic {
                                if is_italic { "1" } else { "0" }
                            } else {
                                &italic
                            },
                        )
                        .replace("{text}", &c.to_string())
                        .replace("{ox}", &format!("{:.2}", x))
                        .replace("{oy}", &format!("{:.2}", y))
                        .replace("{oz}", &format!("{:.2}", z))
                        .replace("{alpha}", &alpha)
                        .replace("{blend}", &blend);

                    creation_infos.push((alias, layer, start, end - start));

                    x += size + kern;
                    layer += 1;
                }
            }

            objects_to_delete.push(obj_handle.clone());
        }

        // Phase 2: Mutate the timeline.
        for (alias, layer, start, length) in creation_infos {
            create_object_from_alias_incremental(edit_section, &alias, layer, start, length);
        }

        for obj_idx in objects_to_delete {
            edit_section.object(&obj_idx).delete_object()?;
        }

        Ok(())
    }
}

fn create_object_from_alias_incremental(
    edit_section: &mut EditSection,
    alias: &str,
    layer: usize,
    frame: usize,
    length: usize,
) {
    if let Err(_) = edit_section.create_object_from_alias(alias, layer, frame, length) {
        create_object_from_alias_incremental(edit_section, alias, layer + 1, frame, length);
    }
}

aviutl2::register_generic_plugin!(TextSplit);
