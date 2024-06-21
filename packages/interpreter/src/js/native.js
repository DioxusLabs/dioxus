function Y(w,H){let B={values:{}},N=H.closest("form");if(N){if(w.type==="input"||w.type==="change"||w.type==="submit"||w.type==="reset"||w.type==="click")B=W(N)}return B}function W(w){const H=new FormData(w),B={};return H.forEach((N,G)=>{if(B[G])B[G].push(N);else B[G]=[N]}),{valid:w.checkValidity(),values:B}}function Z(w){let H=w.selectedOptions,B=[];for(let N=0;N<H.length;N++)B.push(H[N].value);return B}function A(w,H){let B={},N=(G)=>B={...B,...G};if(w instanceof WheelEvent)N(j(w));if(w instanceof MouseEvent)N(F(w));if(w instanceof KeyboardEvent)N(J(w));if(w instanceof InputEvent)N($(w,H));if(w instanceof PointerEvent)N(C(w));if(w instanceof AnimationEvent)N(V(w));if(w instanceof TransitionEvent)N({property_name:w.propertyName,elapsed_time:w.elapsedTime,pseudo_element:w.pseudoElement});if(w instanceof CompositionEvent)N({data:w.data});if(w instanceof DragEvent)N(S(w));if(w instanceof FocusEvent)N({});if(w instanceof ClipboardEvent)N({});if(typeof TouchEvent!=="undefined"&&w instanceof TouchEvent)N(p(w));if(w.type==="submit"||w.type==="reset"||w.type==="click"||w.type==="change"||w.type==="input")N($(w,H));if(w instanceof DragEvent);return B}var $=function(w,H){let B={};if(H instanceof HTMLElement){let N=Y(w,H);B.values=N.values,B.valid=N.valid}if(w.target instanceof HTMLInputElement){let N=w.target,G=N.value??N.textContent??"";if(N.type==="checkbox")G=N.checked?"true":"false";else if(N.type==="radio")G=N.value;B.value=G}if(w.target instanceof HTMLTextAreaElement)B.value=w.target.value;if(w.target instanceof HTMLSelectElement)B.value=Z(w.target).join(",");if(B.value===void 0)B.value="";return B},j=function(w){return{delta_x:w.deltaX,delta_y:w.deltaY,delta_z:w.deltaZ,delta_mode:w.deltaMode}},p=function(w){return{alt_key:w.altKey,ctrl_key:w.ctrlKey,meta_key:w.metaKey,shift_key:w.shiftKey,changed_touches:w.changedTouches,target_touches:w.targetTouches,touches:w.touches}},C=function(w){return{alt_key:w.altKey,button:w.button,buttons:w.buttons,client_x:w.clientX,client_y:w.clientY,ctrl_key:w.ctrlKey,meta_key:w.metaKey,page_x:w.pageX,page_y:w.pageY,screen_x:w.screenX,screen_y:w.screenY,shift_key:w.shiftKey,pointer_id:w.pointerId,width:w.width,height:w.height,pressure:w.pressure,tangential_pressure:w.tangentialPressure,tilt_x:w.tiltX,tilt_y:w.tiltY,twist:w.twist,pointer_type:w.pointerType,is_primary:w.isPrimary}},F=function(w){return{alt_key:w.altKey,button:w.button,buttons:w.buttons,client_x:w.clientX,client_y:w.clientY,ctrl_key:w.ctrlKey,meta_key:w.metaKey,offset_x:w.offsetX,offset_y:w.offsetY,page_x:w.pageX,page_y:w.pageY,screen_x:w.screenX,screen_y:w.screenY,shift_key:w.shiftKey}},J=function(w){return{char_code:w.charCode,is_composing:w.isComposing,key:w.key,alt_key:w.altKey,ctrl_key:w.ctrlKey,meta_key:w.metaKey,key_code:w.keyCode,shift_key:w.shiftKey,location:w.location,repeat:w.repeat,which:w.which,code:w.code}},V=function(w){return{animation_name:w.animationName,elapsed_time:w.elapsedTime,pseudo_element:w.pseudoElement}},S=function(w){let H=void 0;if(w.dataTransfer&&w.dataTransfer.files&&w.dataTransfer.files.length>0)H={files:{placeholder:[]}};return{mouse:{alt_key:w.altKey,ctrl_key:w.ctrlKey,meta_key:w.metaKey,shift_key:w.shiftKey,...F(w)},files:H}};var K=function(w){if(!(w instanceof Node))return null;let H=w,B=null;while(B==null){if(H===null)return null;if(H instanceof Element)B=H.getAttribute("data-dioxus-id");H=H.parentNode}return parseInt(B)},M;if(RawInterpreter!==void 0&&RawInterpreter!==null)M=RawInterpreter;class P extends M{intercept_link_redirects;ipc;editsPath;kickStylesheets;queuedBytes=[];liveview;constructor(w){super();this.editsPath=w,this.kickStylesheets=!1}initialize(w){this.intercept_link_redirects=!0,this.liveview=!1,window.addEventListener("dragover",function(B){if(B.target instanceof Element&&B.target.tagName!="INPUT")B.preventDefault()},!1),window.addEventListener("drop",function(B){if(!(B.target instanceof Element))return;B.preventDefault()},!1),window.addEventListener("click",(B)=>{const N=B.target;if(N instanceof HTMLInputElement&&N.getAttribute("type")==="file"){let G=K(N);if(G!==null){const L=this.serializeIpcMessage("file_dialog",{event:"change&input",accept:N.getAttribute("accept"),directory:N.getAttribute("webkitdirectory")==="true",multiple:N.hasAttribute("multiple"),target:G,bubbles:B.bubbles});this.ipc.postMessage(L),B.preventDefault()}}}),this.ipc=window.ipc;const H=(B)=>this.handleEvent(B,B.type,!0);super.initialize(w,H)}serializeIpcMessage(w,H={}){return JSON.stringify({method:w,params:H})}scrollTo(w,H){const B=this.nodes[w];if(B instanceof HTMLElement)B.scrollIntoView({behavior:H})}getScrollHeight(w){const H=this.nodes[w];if(H instanceof HTMLElement)return H.scrollHeight}getScrollLeft(w){const H=this.nodes[w];if(H instanceof HTMLElement)return H.scrollLeft}getScrollTop(w){const H=this.nodes[w];if(H instanceof HTMLElement)return H.scrollTop}getScrollWidth(w){const H=this.nodes[w];if(H instanceof HTMLElement)return H.scrollWidth}getClientRect(w){const H=this.nodes[w];if(H instanceof HTMLElement){const B=H.getBoundingClientRect();return{type:"GetClientRect",origin:[B.x,B.y],size:[B.width,B.height]}}}setFocus(w,H){const B=this.nodes[w];if(B instanceof HTMLElement)if(H)B.focus();else B.blur()}loadChild(w){let H=this.stack[this.stack.length-1];for(let B=0;B<w.length;B++){let N=w[B];for(H=H.firstChild;N>0;N--)H=H.nextSibling}return H}appendChildren(w,H){const B=this.nodes[w],N=this.stack.splice(this.stack.length-H);for(let G=0;G<H;G++)B.appendChild(N[G])}handleEvent(w,H,B){const N=w.target,G=K(N),L=A(w,N);let Q={name:H,data:L,element:G,bubbles:B};if(this.preventDefaults(w,N),this.liveview){if(N instanceof HTMLInputElement&&(w.type==="change"||w.type==="input")){if(N.getAttribute("type")==="file")this.readFiles(N,L,B,G,H)}}else{const O=this.serializeIpcMessage("user_event",Q);this.ipc.postMessage(O)}}preventDefaults(w,H){let B=null;if(H instanceof Element)B=H.getAttribute("dioxus-prevent-default");if(B&&B.includes(`on${w.type}`))w.preventDefault();if(w.type==="submit")w.preventDefault();if(H instanceof Element&&w.type==="click")this.handleClickNavigate(w,H,B)}handleClickNavigate(w,H,B){if(!this.intercept_link_redirects)return;if(H.tagName==="BUTTON"&&w.type=="submit")w.preventDefault();let N=H.closest("a");if(N==null)return;w.preventDefault();let G=B&&B.includes("onclick"),L=N.getAttribute("dioxus-prevent-default"),Q=L&&L.includes("onclick");if(!G&&!Q){const O=N.getAttribute("href");if(O!==""&&O!==null&&O!==void 0)this.ipc.postMessage(this.serializeIpcMessage("browser_open",{href:O}))}}enqueueBytes(w){this.queuedBytes.push(w)}flushQueuedBytes(){const w=this.queuedBytes;this.queuedBytes=[];for(let H of w)this.run_from_bytes(H)}rafEdits(w,H){if(w)this.run_from_bytes(H),this.waitForRequest(w);else this.enqueueBytes(H),requestAnimationFrame(()=>{this.flushQueuedBytes(),this.waitForRequest(w)})}waitForRequest(w){fetch(new Request(this.editsPath)).then((H)=>H.arrayBuffer()).then((H)=>{this.rafEdits(w,H)})}kickAllStylesheetsOnPage(){let w=document.querySelectorAll("link[rel=stylesheet]");for(let H=0;H<w.length;H++){let B=w[H];fetch(B.href,{cache:"reload"}).then(()=>{B.href=B.href+"?"+Math.random()})}}async readFiles(w,H,B,N,G){let L=w.files,Q={};for(let U=0;U<L.length;U++){const X=L[U];Q[X.name]=Array.from(new Uint8Array(await X.arrayBuffer()))}H.files={files:Q};const O=this.serializeIpcMessage("user_event",{name:G,element:N,data:H,bubbles:B});this.ipc.postMessage(O)}}export{P as NativeInterpreter};
