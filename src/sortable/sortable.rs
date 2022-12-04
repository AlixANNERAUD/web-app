use crate::prelude::*;

#[derive(Properties, PartialEq, Eq)]
pub struct SortableProps {
    pub items: Vec<String>,
}

pub struct Sortable {
    id: usize,
    order: Rc<RefCell<Vec<usize>>>,
    reload_id: usize,
    currently_dragged: Rc<RefCell<Option<(usize, i32, Vec<i32>)>>>,
    on_mouse_down: wasm_bindgen::prelude::Closure<dyn std::ops::FnMut(web_sys::MouseEvent)>,
    on_mouse_move: wasm_bindgen::prelude::Closure<dyn std::ops::FnMut(web_sys::MouseEvent)>,
    on_mouse_up: wasm_bindgen::prelude::Closure<dyn std::ops::FnMut(web_sys::Event)>,
}

pub enum SortableMsg {
    ChangeOrder(Vec<usize>),
}

impl Component for Sortable {
    type Message = SortableMsg;
    type Properties = SortableProps;

    fn create(ctx: &Context<Self>) -> Self {
        let id = (js_sys::Math::random() * 1_000_000.0) as usize;
        let w = window();
        let item_count = ctx.props().items.len();
        let order: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new((0..item_count).collect()));
        let currently_dragged: Rc<RefCell<Option<(usize, i32, Vec<i32>)>>> = Rc::new(RefCell::new(None));

        let currently_dragged2 = currently_dragged.clone();
        let doc = w.doc();
        let link2 = ctx.link().clone();
        let release_drag = move || {
            if currently_dragged2.borrow_mut().take().is_some() {
                let mut rects = Vec::new();
                for i in 0..item_count {
                    let fid = format!("sortable-{id}-{i}");
                    let el = doc.get_element_by_id(&fid).unwrap();
                    let rect = el.get_bounding_client_rect();
                    rects.push((i, rect));
                }
                rects.sort_by_key(|(_, rect)| (rect.top() as i32 + rect.height() as i32) / 2);
                let new_order = rects.into_iter().map(|(i, _)| i).collect::<Vec<_>>();
                link2.send_message(SortableMsg::ChangeOrder(new_order));
            }
        };
        
        let doc = w.doc();
        let currently_dragged2 = currently_dragged.clone();
        let release_drag2 = release_drag.clone();
        let on_mouse_down = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
            release_drag2();

            let x = e.client_x();
            let y = e.client_y();

            let mut centers = Vec::new();
            let mut dragged = None;
            for i in 0..item_count {
                let fid = format!("sortable-{id}-{i}");
                let el = doc.get_element_by_id(&fid).unwrap();
                let rect = el.get_bounding_client_rect();
                let top = rect.top() as i32;
                let bottom = rect.bottom() as i32;
                if x >= rect.left() as i32 && x <= rect.right() as i32 && y >= top && y <= bottom {
                    dragged = Some(i);
                }
                centers.push((bottom + top) / 2);
            }
            if let Some(dragged) = dragged {
                currently_dragged2.borrow_mut().replace((dragged, y, centers));
            }
        }) as Box<dyn FnMut(_)>);
        w.add_event_listener_with_callback("mousedown", on_mouse_down.as_ref().unchecked_ref()).unwrap();

        let doc = w.doc();
        let currently_dragged2 = currently_dragged.clone();
        let ordered2 = order.clone();
        let on_mouse_move = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
            if let Some((i, y, centers)) = currently_dragged2.borrow().as_ref() {
                let dy = e.client_y() - y;
                let fid = format!("sortable-{id}-{i}");
                let el = doc.get_element_by_id(&fid).unwrap();
                el.set_attribute("style", &format!("transition: unset; top: {dy}px;")).unwrap();
                let rect = el.get_bounding_client_rect();
                let top = rect.top() as i32;
                let bottom = rect.bottom() as i32;
                let height = bottom - top;

                let position = ordered2.borrow().deref().iter().position(|&x| x == *i).unwrap();
                for other in 0..item_count {
                    let other_position = ordered2.borrow().deref().iter().position(|&x| x == other).unwrap();
                    if other_position == position { continue; }
                    let other_item_el = doc.get_element_by_id(&format!("sortable-{id}-{other}")).unwrap();
                    let center = centers.get(other).unwrap();
                    if other_position > position && bottom > *center {
                        other_item_el.set_attribute("style", &format!("top: calc(-{height}px - 0.5rem);")).unwrap();
                    } else if other_position < position && top < *center {
                        other_item_el.set_attribute("style", &format!("top: calc({height}px + 0.5rem);")).unwrap();
                    } else {
                        other_item_el.set_attribute("style", "top: 0px;").unwrap();
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);
        w.add_event_listener_with_callback("mousemove", on_mouse_move.as_ref().unchecked_ref()).unwrap();

        let on_mouse_up = Closure::wrap(Box::new(move |_: web_sys::Event| {
            release_drag();
        }) as Box<dyn FnMut(_)>);
        w.add_event_listener_with_callback("mouseup", on_mouse_up.as_ref().unchecked_ref()).unwrap();

        Self {
            id,
            order,
            reload_id: 0,
            currently_dragged,
            on_mouse_down,
            on_mouse_move,
            on_mouse_up,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            SortableMsg::ChangeOrder(order) => {
                self.order.replace(order);
                self.reload_id += 1;
                true
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        *self = Self::create(ctx);
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let items = self.order.borrow().iter().map(|i| {
            let item = ctx.props().items.get(*i).unwrap();
            let fid = format!("sortable-{}-{}", self.id, i);
            html! {
                <div class="sortable-item" id={fid} style={format!("top: 0px; transition: unset; reload-id: {};", self.reload_id)}>
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20"><path d="M2 11h16v2H2zm0-4h16v2H2zm8 11l3-3H7l3 3zm0-16L7 5h6l-3-3z"/></svg>
                    {item}
                </div>
            }
        }).collect::<Html>();

        html! {
            <div class="sortable">
                {items}
            </div>
        }
    }
}

impl Drop for Sortable {
    fn drop(&mut self) {
        let w = window();
        let _ = w.remove_event_listener_with_callback("mousedown", self.on_mouse_down.as_ref().unchecked_ref());
        let _ = w.remove_event_listener_with_callback("mousemove", self.on_mouse_move.as_ref().unchecked_ref());
        let _ = w.remove_event_listener_with_callback("mouseup", self.on_mouse_up.as_ref().unchecked_ref());
    }
}
