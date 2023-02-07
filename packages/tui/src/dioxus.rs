pub fn launch(app: Component<()>) {
    launch_cfg(app, Config::default())
}

pub fn launch_cfg(app: Component<()>, cfg: Config) {
    launch_cfg_with_props(app, (), cfg);
}

pub fn launch_cfg_with_props<Props: 'static>(app: Component<Props>, props: Props, cfg: Config) {
    let mut dom = VirtualDom::new_with_props(app, props);
    let mut rdom = RealDom::new(Box::new([
        TaffyLayout::to_type_erased(),
        Focus::to_type_erased(),
        StyleModifier::to_type_erased(),
        PreventDefault::to_type_erased(),
    ]));

    let (handler, state, register_event) = RinkInputHandler::craete(&mut rdom);

    // Setup input handling
    let (event_tx, event_rx) = unbounded();
    let event_tx_clone = event_tx.clone();
    if !cfg.headless {
        std::thread::spawn(move || {
            let tick_rate = Duration::from_millis(1000);
            loop {
                if crossterm::event::poll(tick_rate).unwrap() {
                    let evt = crossterm::event::read().unwrap();
                    if event_tx.unbounded_send(InputEvent::UserInput(evt)).is_err() {
                        break;
                    }
                }
            }
        });
    }

    let cx = dom.base_scope();
    let rdom = Rc::new(RefCell::new(rdom));
    let taffy = Arc::new(Mutex::new(Taffy::new()));
    cx.provide_context(state);
    cx.provide_context(TuiContext { tx: event_tx_clone });
    cx.provide_context(Query {
        rdom: rdom.clone(),
        stretch: taffy.clone(),
    });

    {
        let mut rdom = rdom.borrow_mut();
        let mutations = dom.rebuild();
        rdom.apply_mutations(mutations);
        let mut any_map = SendAnyMap::new();
        any_map.insert(taffy.clone());
        let _ = rdom.update_state(any_map, false);
    }

    render_vdom(
        &mut dom,
        event_rx,
        handler,
        cfg,
        rdom,
        taffy,
        register_event,
    )
    .unwrap();
}
