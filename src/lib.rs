mod parser;

use parser::parse_markup;

use aviutl2::{
    AnyResult,
    anyhow::bail,
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
        //let current_object = edit_section.get_focused_object()?;
        let selected_objects = edit_section.get_selected_objects()?;
        let mut objects_to_delete = Vec::new();
        for _obj in selected_objects {
            let obj = edit_section.object(&_obj);

            let Ok(text) = obj.get_effect_item("テキスト", 0, "テキスト") else {
                bail!("選択されたオブジェクトはテキストオブジェクトではありません。");
            };

            let Ok(elements) = parse_markup(&text) else {
                bail!("テキストの解析に失敗しました: {}", text);
            };

            let layer_frame = obj.get_layer_frame()?;

            let layer = layer_frame.layer + 1;
            let start = layer_frame.start;
            let end = layer_frame.end;

            let size = obj.get_effect_item("テキスト", 0, "サイズ")?;
            let font = obj.get_effect_item("テキスト", 0, "フォント")?;
            let color = obj.get_effect_item("テキスト", 0, "文字色")?;
            let subcolor = obj.get_effect_item("テキスト", 0, "影・縁色")?;
            let style = obj.get_effect_item("テキスト", 0, "文字装飾")?;
            let bold = obj.get_effect_item("テキスト", 0, "B")?;
            let italic = obj.get_effect_item("テキスト", 0, "I")?;

            for el in elements {
                for c in el.text.chars() {
                    let alias = format!(
                        "[Object]
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
X=0.00
Y=0.00
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
",
                        start = start,
                        end = end,
                        size = el.size.map_or_else(|| size.clone(), |s| s.to_string()),
                        font = el.font.clone().map_or_else(|| font.clone(), |f| f),
                        color = el.color.clone().map_or_else(|| color.clone(), |c| c),
                        subcolor = subcolor,
                        style = style,
                        text = c.to_string(),
                        bold = if let Some(b) = el.is_bold {
                            if b { "1" } else { "0" }
                        } else {
                            &bold
                        },
                        italic = if let Some(i) = el.is_italic {
                            if i { "1" } else { "0" }
                        } else {
                            &italic
                        },
                    );

                    create_object_from_alias_incremental(
                        edit_section,
                        &alias,
                        layer,
                        start,
                        end - start,
                    );
                }
            }

            objects_to_delete.push(_obj);
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
