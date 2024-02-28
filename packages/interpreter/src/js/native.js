var __extends = (this && this.__extends) || (function () {
    var extendStatics = function (d, b) {
        extendStatics = Object.setPrototypeOf ||
            ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
            function (d, b) { for (var p in b) if (Object.prototype.hasOwnProperty.call(b, p)) d[p] = b[p]; };
        return extendStatics(d, b);
    };
    return function (d, b) {
        if (typeof b !== "function" && b !== null)
            throw new TypeError("Class extends value " + String(b) + " is not a constructor or null");
        extendStatics(d, b);
        function __() { this.constructor = d; }
        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
    };
})();
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;
    return g = { next: verb(0), "throw": verb(1), "return": verb(2) }, typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (g && (g = 0, op[0] && (_ = 0)), _) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
};
System.register("form", [], function (exports_1, context_1) {
    "use strict";
    var __moduleName = context_1 && context_1.id;
    function retriveValues(event, target) {
        var contents = {};
        if (target instanceof HTMLFormElement && (event.type === "submit" || event.type === "input")) {
            retrieveFormValues(target, contents);
        }
        if (target instanceof HTMLSelectElement && (event.type === "input" || event.type === "change")) {
            retriveInputsValues(target, contents);
        }
        return contents;
    }
    exports_1("retriveValues", retriveValues);
    function retrieveFormValues(form, contents) {
        var formData = new FormData(form);
        for (var name_1 in formData.keys()) {
            var element = form.elements.namedItem(name_1);
            if (!(element instanceof HTMLInputElement)) {
                continue;
            }
            switch (element.type) {
                case "select-multiple":
                    contents[name_1] = formData.getAll(name_1);
                    break;
                default:
                    contents[name_1] = [formData.get(name_1)];
                    break;
            }
        }
    }
    exports_1("retrieveFormValues", retrieveFormValues);
    function retriveInputsValues(target, contents) {
        var selectData = target.options;
        contents["options"] = [];
        for (var i = 0; i < selectData.length; i++) {
            var option = selectData[i];
            if (option.selected) {
                contents["options"].push(option.value.toString());
            }
        }
    }
    exports_1("retriveInputsValues", retriveInputsValues);
    return {
        setters: [],
        execute: function () {
        }
    };
});
System.register("interpreter_core", [], function (exports_2, context_2) {
    "use strict";
    var Interpreter;
    var __moduleName = context_2 && context_2.id;
    return {
        setters: [],
        execute: function () {
            Interpreter = (function () {
                function Interpreter(root, handler) {
                    this.root = root;
                    this.nodes = [root];
                    this.stack = [root];
                    this.global = {};
                    this.local = {};
                    this.handler = handler;
                }
                Interpreter.prototype.createListener = function (event_name, element, bubbles) {
                    if (bubbles) {
                        if (this.global[event_name] === undefined) {
                            this.global[event_name] = { active: 1, callback: this.handler };
                            this.root.addEventListener(event_name, this.handler);
                        }
                        else {
                            this.global[event_name].active++;
                        }
                    }
                    else {
                        var id = element.getAttribute("data-dioxus-id");
                        if (!this.local[id]) {
                            this.local[id] = {};
                        }
                        element.addEventListener(event_name, this.handler);
                    }
                };
                Interpreter.prototype.removeListener = function (element, event_name, bubbles) {
                    if (bubbles) {
                        this.removeBubblingListener(event_name);
                    }
                    else {
                        this.removeNonBubblingListener(element, event_name);
                    }
                };
                Interpreter.prototype.removeBubblingListener = function (event_name) {
                    this.global[event_name].active--;
                    if (this.global[event_name].active === 0) {
                        this.root.removeEventListener(event_name, this.global[event_name].callback);
                        delete this.global[event_name];
                    }
                };
                Interpreter.prototype.removeNonBubblingListener = function (element, event_name) {
                    var id = element.getAttribute("data-dioxus-id");
                    delete this.local[id][event_name];
                    if (Object.keys(this.local[id]).length === 0) {
                        delete this.local[id];
                    }
                    element.removeEventListener(event_name, this.handler);
                };
                Interpreter.prototype.removeAllNonBubblingListeners = function (element) {
                    var id = element.getAttribute("data-dioxus-id");
                    delete this.local[id];
                };
                return Interpreter;
            }());
            exports_2("Interpreter", Interpreter);
        }
    };
});
System.register("serialize", [], function (exports_3, context_3) {
    "use strict";
    var __moduleName = context_3 && context_3.id;
    function serializeEvent(event) {
        var _a;
        if (event instanceof InputEvent) {
            if (event.target instanceof HTMLInputElement) {
                var target = event.target;
                var value = (_a = target.value) !== null && _a !== void 0 ? _a : target.textContent;
                if (target.type === "checkbox") {
                    value = target.checked ? "true" : "false";
                }
                return {
                    value: value,
                    values: {},
                };
            }
            return {};
        }
        if (event instanceof KeyboardEvent) {
            return {
                char_code: event.charCode,
                is_composing: event.isComposing,
                key: event.key,
                alt_key: event.altKey,
                ctrl_key: event.ctrlKey,
                meta_key: event.metaKey,
                key_code: event.keyCode,
                shift_key: event.shiftKey,
                location: event.location,
                repeat: event.repeat,
                which: event.which,
                code: event.code,
            };
        }
        if (event instanceof MouseEvent) {
            return {
                alt_key: event.altKey,
                button: event.button,
                buttons: event.buttons,
                client_x: event.clientX,
                client_y: event.clientY,
                ctrl_key: event.ctrlKey,
                meta_key: event.metaKey,
                offset_x: event.offsetX,
                offset_y: event.offsetY,
                page_x: event.pageX,
                page_y: event.pageY,
                screen_x: event.screenX,
                screen_y: event.screenY,
                shift_key: event.shiftKey,
            };
        }
        if (event instanceof PointerEvent) {
            return {
                alt_key: event.altKey,
                button: event.button,
                buttons: event.buttons,
                client_x: event.clientX,
                client_y: event.clientY,
                ctrl_key: event.ctrlKey,
                meta_key: event.metaKey,
                page_x: event.pageX,
                page_y: event.pageY,
                screen_x: event.screenX,
                screen_y: event.screenY,
                shift_key: event.shiftKey,
                pointer_id: event.pointerId,
                width: event.width,
                height: event.height,
                pressure: event.pressure,
                tangential_pressure: event.tangentialPressure,
                tilt_x: event.tiltX,
                tilt_y: event.tiltY,
                twist: event.twist,
                pointer_type: event.pointerType,
                is_primary: event.isPrimary,
            };
        }
        if (event instanceof TouchEvent) {
            return {
                alt_key: event.altKey,
                ctrl_key: event.ctrlKey,
                meta_key: event.metaKey,
                shift_key: event.shiftKey,
                changed_touches: event.changedTouches,
                target_touches: event.targetTouches,
                touches: event.touches,
            };
        }
        if (event instanceof WheelEvent) {
            return {
                delta_x: event.deltaX,
                delta_y: event.deltaY,
                delta_z: event.deltaZ,
                delta_mode: event.deltaMode,
            };
        }
        if (event instanceof AnimationEvent) {
            return {
                animation_name: event.animationName,
                elapsed_time: event.elapsedTime,
                pseudo_element: event.pseudoElement,
            };
        }
        if (event instanceof TransitionEvent) {
            return {
                property_name: event.propertyName,
                elapsed_time: event.elapsedTime,
                pseudo_element: event.pseudoElement,
            };
        }
        if (event instanceof ClipboardEvent) {
            return {};
        }
        if (event instanceof CompositionEvent) {
            return {
                data: event.data,
            };
        }
        if (event instanceof DragEvent) {
            return {
                mouse: {
                    alt_key: event.altKey,
                    ctrl_key: event.ctrlKey,
                    meta_key: event.metaKey,
                    shift_key: event.shiftKey,
                },
                files: [],
            };
        }
        if (event instanceof FocusEvent) {
            return {};
        }
        return {};
    }
    exports_3("serializeEvent", serializeEvent);
    return {
        setters: [],
        execute: function () {
        }
    };
});
System.register("interpreter_native", ["form", "interpreter_core", "serialize"], function (exports_4, context_4) {
    "use strict";
    var form_1, interpreter_core_1, serialize_1, NativeInterpreter;
    var __moduleName = context_4 && context_4.id;
    function handleVirtualdomEventSync(contents) {
        var xhr = new XMLHttpRequest();
        xhr.timeout = 1000;
        xhr.open("GET", "/handle/event.please", false);
        xhr.setRequestHeader("Content-Type", "application/json");
        xhr.send(contents);
        return JSON.parse(xhr.responseText);
    }
    function targetId(target) {
        if (!(target instanceof Node)) {
            return null;
        }
        var ourTarget = target;
        var realId = null;
        while (realId == null) {
            if (ourTarget === null) {
                return null;
            }
            if (ourTarget instanceof Element) {
                realId = ourTarget.getAttribute("data-dioxus-id");
            }
            ourTarget = ourTarget.parentNode;
        }
        return parseInt(realId);
    }
    return {
        setters: [
            function (form_1_1) {
                form_1 = form_1_1;
            },
            function (interpreter_core_1_1) {
                interpreter_core_1 = interpreter_core_1_1;
            },
            function (serialize_1_1) {
                serialize_1 = serialize_1_1;
            }
        ],
        execute: function () {
            NativeInterpreter = (function (_super) {
                __extends(NativeInterpreter, _super);
                function NativeInterpreter(root) {
                    var _this = _super.call(this, root, function (event) { return _this.handleEvent(event, event.type, true); }) || this;
                    _this.intercept_link_redirects = true;
                    _this.liveview = false;
                    _this.ipc = window.ipc;
                    return _this;
                }
                NativeInterpreter.prototype.serializeIpcMessage = function (method, params) {
                    if (params === void 0) { params = {}; }
                    return JSON.stringify({ method: method, params: params });
                };
                NativeInterpreter.prototype.scrollTo = function (id, behavior) {
                    var node = this.nodes[id];
                    if (node instanceof HTMLElement) {
                        node.scrollIntoView({ behavior: behavior });
                    }
                };
                NativeInterpreter.prototype.getClientRect = function (id) {
                    var node = this.nodes[id];
                    if (node instanceof HTMLElement) {
                        var rect = node.getBoundingClientRect();
                        return {
                            type: "GetClientRect",
                            origin: [rect.x, rect.y],
                            size: [rect.width, rect.height],
                        };
                    }
                };
                NativeInterpreter.prototype.setFocus = function (id, focus) {
                    var node = this.nodes[id];
                    if (node instanceof HTMLElement) {
                        if (focus) {
                            node.focus();
                        }
                        else {
                            node.blur();
                        }
                    }
                };
                NativeInterpreter.prototype.LoadChild = function (array) {
                    var node = this.stack[this.stack.length - 1];
                    for (var i = 0; i < array.length; i++) {
                        var end = array[i];
                        for (node = node.firstChild; end > 0; end--) {
                            node = node.nextSibling;
                        }
                    }
                    return node;
                };
                NativeInterpreter.prototype.AppendChildren = function (id, many) {
                    var root = this.nodes[id];
                    var els = this.stack.splice(this.stack.length - many);
                    for (var k = 0; k < many; k++) {
                        root.appendChild(els[k]);
                    }
                };
                NativeInterpreter.prototype.handleEvent = function (event, name, bubbles) {
                    var target = event.target;
                    var realId = targetId(target);
                    var contents = serialize_1.serializeEvent(event);
                    if (target instanceof HTMLElement) {
                        contents.values = form_1.retriveValues(event, target);
                    }
                    var body = {
                        name: name,
                        data: contents,
                        element: realId,
                        bubbles: bubbles,
                    };
                    this.preventDefaults(event, target);
                    if (this.liveview) {
                        if (target instanceof HTMLInputElement && (event.type === "change" || event.type === "input")) {
                            if (target.getAttribute("type") === "file") {
                                this.readFiles(target, contents, bubbles, realId, name);
                            }
                        }
                    }
                    else {
                        var res = handleVirtualdomEventSync(JSON.stringify(body));
                        if (res.preventDefault) {
                            event.preventDefault();
                        }
                        if (res.stopPropagation) {
                            event.stopPropagation();
                        }
                    }
                };
                NativeInterpreter.prototype.readFiles = function (target, contents, bubbles, realId, name) {
                    return __awaiter(this, void 0, void 0, function () {
                        var files, file_contents, i, file, _a, _b, _c, _d, _e, message;
                        return __generator(this, function (_f) {
                            switch (_f.label) {
                                case 0:
                                    files = target.files;
                                    file_contents = {};
                                    i = 0;
                                    _f.label = 1;
                                case 1:
                                    if (!(i < files.length)) return [3, 4];
                                    file = files[i];
                                    _a = file_contents;
                                    _b = file.name;
                                    _d = (_c = Array).from;
                                    _e = Uint8Array.bind;
                                    return [4, file.arrayBuffer()];
                                case 2:
                                    _a[_b] = _d.apply(_c, [new (_e.apply(Uint8Array, [void 0, _f.sent()]))()]);
                                    _f.label = 3;
                                case 3:
                                    i++;
                                    return [3, 1];
                                case 4:
                                    contents.files = { files: file_contents };
                                    message = this.serializeIpcMessage("user_event", {
                                        name: name,
                                        element: realId,
                                        data: contents,
                                        bubbles: bubbles,
                                    });
                                    this.ipc.postMessage(message);
                                    return [2];
                            }
                        });
                    });
                };
                NativeInterpreter.prototype.preventDefaults = function (event, target) {
                    var preventDefaultRequests = null;
                    if (target instanceof Element) {
                        preventDefaultRequests = target.getAttribute("dioxus-prevent-default");
                    }
                    if (preventDefaultRequests && preventDefaultRequests.includes("on".concat(event.type))) {
                        event.preventDefault();
                    }
                    if (event.type === "submit") {
                        event.preventDefault();
                    }
                    if (target instanceof Element && event.type === "click") {
                        this.handleClickNavigate(event, target, preventDefaultRequests);
                    }
                };
                NativeInterpreter.prototype.handleClickNavigate = function (event, target, preventDefaultRequests) {
                    if (!this.intercept_link_redirects) {
                        return;
                    }
                    if (target.tagName === "BUTTON" && event.type == "submit") {
                        event.preventDefault();
                    }
                    var a_element = target.closest("a");
                    if (a_element == null) {
                        return;
                    }
                    event.preventDefault();
                    var elementShouldPreventDefault = preventDefaultRequests && preventDefaultRequests.includes("onclick");
                    var aElementShouldPreventDefault = a_element.getAttribute("dioxus-prevent-default");
                    var linkShouldPreventDefault = aElementShouldPreventDefault &&
                        aElementShouldPreventDefault.includes("onclick");
                    if (!elementShouldPreventDefault && !linkShouldPreventDefault) {
                        var href = a_element.getAttribute("href");
                        if (href !== "" && href !== null && href !== undefined) {
                            this.ipc.postMessage(this.serializeIpcMessage("browser_open", { href: href }));
                        }
                    }
                };
                return NativeInterpreter;
            }(interpreter_core_1.Interpreter));
            exports_4("NativeInterpreter", NativeInterpreter);
        }
    };
});
