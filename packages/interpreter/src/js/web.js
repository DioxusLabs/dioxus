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
System.register("interpreter_core", [], function (exports_1, context_1) {
    "use strict";
    var Interpreter;
    var __moduleName = context_1 && context_1.id;
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
                Interpreter.prototype.getNode = function (id) {
                    return this.nodes[id];
                };
                Interpreter.prototype.appendChildren = function (id, many) {
                    var root = this.nodes[id];
                    var els = this.stack.splice(this.stack.length - many);
                    for (var k = 0; k < many; k++) {
                        root.appendChild(els[k]);
                    }
                };
                return Interpreter;
            }());
            exports_1("Interpreter", Interpreter);
        }
    };
});
System.register("interpreter_web", ["interpreter_core"], function (exports_2, context_2) {
    "use strict";
    var interpreter_core_1, WebInterpreter;
    var __moduleName = context_2 && context_2.id;
    return {
        setters: [
            function (interpreter_core_1_1) {
                interpreter_core_1 = interpreter_core_1_1;
            }
        ],
        execute: function () {
            WebInterpreter = (function (_super) {
                __extends(WebInterpreter, _super);
                function WebInterpreter(root, handler) {
                    return _super.call(this, root, handler) || this;
                }
                WebInterpreter.prototype.LoadChild = function (ptr, len) {
                    var node = this.stack[this.stack.length - 1];
                    var ptr_end = ptr + len;
                    for (; ptr < ptr_end; ptr++) {
                        var end = this.m.getUint8(ptr);
                        for (node = node.firstChild; end > 0; end--) {
                            node = node.nextSibling;
                        }
                    }
                    return node;
                };
                WebInterpreter.prototype.saveTemplate = function (nodes, tmpl_id) {
                    this.templates[tmpl_id] = nodes;
                };
                WebInterpreter.prototype.hydrateRoot = function (ids) {
                    var hydrateNodes = document.querySelectorAll('[data-node-hydration]');
                    for (var i = 0; i < hydrateNodes.length; i++) {
                        var hydrateNode = hydrateNodes[i];
                        var hydration = hydrateNode.getAttribute('data-node-hydration');
                        var split = hydration.split(',');
                        var id = ids[parseInt(split[0])];
                        this.nodes[id] = hydrateNode;
                        if (split.length > 1) {
                            hydrateNode.listening = split.length - 1;
                            hydrateNode.setAttribute('data-dioxus-id', id.toString());
                            for (var j = 1; j < split.length; j++) {
                                var listener = split[j];
                                var split2 = listener.split(':');
                                var event_name = split2[0];
                                var bubbles = split2[1] === '1';
                                this.createListener(event_name, hydrateNode, bubbles);
                            }
                        }
                    }
                    var treeWalker = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT);
                    var currentNode = treeWalker.nextNode();
                    while (currentNode) {
                        var id = currentNode.textContent;
                        var split = id.split('node-id');
                        if (split.length > 1) {
                            this.nodes[ids[parseInt(split[1])]] = currentNode.nextSibling;
                        }
                        currentNode = treeWalker.nextNode();
                    }
                };
                return WebInterpreter;
            }(interpreter_core_1.Interpreter));
            exports_2("WebInterpreter", WebInterpreter);
        }
    };
});
