# VSCode support for Dioxus html macro

This macro provides syntax highlighting for the html! macro used in Dioxus projects. Users should feel at home writing html and css alongside the custom attributes used by Dioxus.


## How it works
This extension works by:
- Creating a custom HTML ruleset for dioxus html! nodes
- Request forwarding content


## Resources
Request forwarding is performed intelligently by the extension. 
Requests within the html! tag are forwarded to the html language service. It's simple and doesn't 

https://code.visualstudio.com/api/language-extensions/embedded-languages#language-services

