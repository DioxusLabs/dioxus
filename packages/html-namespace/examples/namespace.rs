use scraper::{Html, Selector};

struct AttrClass {
    attr_name: String,
    attr_doc_link: String,
    target_elements: Vec<String>,
    docs: String,
}

const contents: &str = "";
// let contents = include_str!("./attrlist.html");
fn main() {
    let mut items: Vec<AttrClass> = Vec::new();

    let fragment = Html::parse_fragment(contents);

    let ul_selector = Selector::parse("tbody").unwrap();
    let li_selector = Selector::parse("tr").unwrap();

    let ul = fragment.select(&ul_selector).next().unwrap();
    for element in ul.select(&li_selector) {
        let mut childs = element.children().into_iter();

        let attr_field_node = childs.next().unwrap();
        let elements_node = childs.next().unwrap();
        let description_node = childs.next().unwrap();

        // let attr_name = { todo!() };
        // let attr_doc_link = { todo!() };
        // let target_elements = { todo!() };

        // let docs = description_node.text();

        // todo!()
        // items.push(AttrClass {
        //     attr_name,
        //     attr_doc_link,
        //     target_elements,
        //     docs,
        // })
    }
}
