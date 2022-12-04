use crate::prelude::*;

#[derive(Properties, PartialEq, Eq)]
pub struct SortableProps {
    pub items: Vec<String>,
}

pub struct Sortable {
    id: usize,
    ordered: Vec<usize>,
    currently_dragged: Rc<RefCell<Option<(usize, i32)>>>,
    on_mouse_down: wasm_bindgen::prelude::Closure<dyn std::ops::FnMut(web_sys::MouseEvent)>,
    on_mouse_move: wasm_bindgen::prelude::Closure<dyn std::ops::FnMut(web_sys::MouseEvent)>,
    on_mouse_up: wasm_bindgen::prelude::Closure<dyn std::ops::FnMut(web_sys::Event)>,
}

pub enum SortableMsg {
    Reload,
    //DragStart(usize,)
}

impl Component for Sortable {
    type Message = SortableMsg;
    type Properties = SortableProps;

    fn create(ctx: &Context<Self>) -> Self {
        let id = (js_sys::Math::random() * 1_000_000.0) as usize;
        let w = window();
        let item_count = ctx.props().items.len();

        let currently_dragged = Rc::new(RefCell::new(None));

        let currently_dragged2 = currently_dragged.clone();
        let doc = w.doc();
        let release_drag = move || {
            if let Some((i, _y)) = currently_dragged2.borrow_mut().take() {
                let fid = format!("sortable-{id}-{i}");
                let el = doc.get_element_by_id(&fid).unwrap();
                el.set_attribute("style", "top: 0px;").unwrap();
            }
        };
        
        let doc = w.doc();
        let currently_dragged2 = currently_dragged.clone();
        let release_drag2 = release_drag.clone();
        let on_mouse_down = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
            release_drag2();
            
            let x = e.client_x();
            let y = e.client_y();

            for i in 0..item_count {
                let fid = format!("sortable-{id}-{i}");
                let el = doc.get_element_by_id(&fid).unwrap();
                let rect = el.get_bounding_client_rect();
                if x >= rect.left() as i32 && x <= rect.right() as i32 && y >= rect.top() as i32 && y <= rect.bottom() as i32 {
                    log!("{i} is dragged");
                    currently_dragged2.borrow_mut().replace((i, y));
                    return;
                }
            }
        }) as Box<dyn FnMut(_)>);
        w.add_event_listener_with_callback("mousedown", on_mouse_down.as_ref().unchecked_ref()).unwrap();

        let doc = w.doc();
        let currently_dragged2 = currently_dragged.clone();
        let on_mouse_move = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
            if let Some((i, y)) = currently_dragged2.borrow().as_ref() {
                let dy = e.client_y() - y;
                let fid = format!("sortable-{id}-{i}");
                let el = doc.get_element_by_id(&fid).unwrap();
                el.set_attribute("style", &format!("top: {dy}px;")).unwrap();
            }
        }) as Box<dyn FnMut(_)>);
        w.add_event_listener_with_callback("mousemove", on_mouse_move.as_ref().unchecked_ref()).unwrap();

        let on_mouse_up = Closure::wrap(Box::new(move |_: web_sys::Event| {
            release_drag();
        }) as Box<dyn FnMut(_)>);

        Self {
            id,
            ordered: (0..ctx.props().items.len()).collect(),
            currently_dragged,
            on_mouse_down,
            on_mouse_move,
            on_mouse_up,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            SortableMsg::Reload => {
                true
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        *self = Self::create(ctx);
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let items = self.ordered.iter().map(|i| {
            let item = ctx.props().items.get(*i).unwrap();
            let fid = format!("sortable-{}-{}", self.id, i);
            html! {
                <div class="sortable-item" id={fid}>
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
