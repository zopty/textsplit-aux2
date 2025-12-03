mod parser;

use parser::parse_markup;

use aviutl2::{
    AnyResult,
    anyhow::{self, Context},
    generic::{EditSection, GenericPlugin},
    log,
};

const TEXT_ALIAS_TEMPLATE: &'static str = "[Object]
frame={start},{end}
[Object.0]
effect.name=テキスト
サイズ={size}
字間=0.00
行間=0.00
表示速度=0.00
フォント={font}
文字色={color}
影・縁色={subcolor}
文字装飾={style}
文字揃え=左寄せ[上]
B={bold}
I={italic}
テキスト={text}
文字毎に個別オブジェクト=0
自動スクロール=0
移動座標上に表示=0
オブジェクトの長さを自動調節=0
[Object.1]
effect.name=標準描画
X={ox}
Y={oy}
Z=0.00
Group=1
中心X=0.00
中心Y=0.00
中心Z=0.00
X軸回転=0.00
Y軸回転=0.00
Z軸回転=0.00
拡大率=100.000
縦横比=0.000
透明度=0.00
合成モード=通常
";

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

            let text: String = obj
                .get_effect_item("テキスト", 0, "テキスト")
                .context("選択されたオブジェクトはテキストオブジェクトではありません。")?
                .replace("\\n", "\n");

            let elements = parse_markup(&text)
                .map_err(|e| anyhow::anyhow!("テキストの解析に失敗しました: {}: {}", text, e))?;

            let layer_frame = obj.get_layer_frame()?;
            let _layer = layer_frame.layer + 1;
            let start = layer_frame.start;
            let end = layer_frame.end;

            let _size: f32 = obj.get_effect_item("テキスト", 0, "サイズ")?.parse()?;
            let font = obj.get_effect_item("テキスト", 0, "フォント")?;
            let color = obj.get_effect_item("テキスト", 0, "文字色")?;
            let subcolor = obj.get_effect_item("テキスト", 0, "影・縁色")?;
            let style = obj.get_effect_item("テキスト", 0, "文字装飾")?;
            let bold = obj.get_effect_item("テキスト", 0, "B")?;
            let italic = obj.get_effect_item("テキスト", 0, "I")?;
            let _x: f32 = obj.get_effect_item("標準描画", 0, "X")?.parse()?;
            let _y: f32 = obj.get_effect_item("標準描画", 0, "Y")?.parse()?;

            let mut x = _x;
            let mut y = _y;
            let mut layer = _layer;
            for el in elements {
                log::info!("{}", el.text);
                if el.text == "\n" {
                    x = _x;
                    y += _size;
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
                        .replace("{oy}", &format!("{:.2}", y));

                    creation_infos.push((alias, layer, start, end - start));

                    x += size;
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
