var createElementInHead=function(tag,attributes,children){console.log("creating element in head",tag,attributes,children);const element=document.createElement(tag);for(let[key,value]of attributes)element.setAttribute(key,value);if(children)element.appendChild(document.createTextNode(children));document.head.appendChild(element)};window.createElementInHead=createElementInHead;
