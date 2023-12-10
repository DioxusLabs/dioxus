// Dioxus-CLI
// https://github.com/DioxusLabs/cli

(function () {
    var protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    var url = protocol + '//' + window.location.host + '/_dioxus/ws';
    var poll_interval = 8080;
    var reload_upon_connect = () => {
        window.setTimeout(
            () => {
                var ws = new WebSocket(url);
                ws.onopen = () => window.location.reload();
                ws.onclose = reload_upon_connect;
            },
            poll_interval);
    };

    var ws = new WebSocket(url);
    ws.onmessage = (ev) => {
        if (ev.data.method == "reload") {
            window.location.reload();
        }
        if (ev.data.method == "refresh_asset") {
            const url = ev.data.url;
            const random_query_param = "?dioxus=" + Math.random().toString(36).substring(7);
            // Look for any urls that match the one we got from the server and add a random query param to it to force a reload
            // Go through every element attribute
            for (const element of document.querySelectorAll("*")) {
                for (const attribute of element.getAttributeNames()) {
                    if (element.getAttribute(attribute).includes(url)) {
                        // Handle cases where there is more than the single url in the attribute
                        const attribute_value = element.getAttribute(attribute);
                        const attribute_value_parts = attribute_value.split(url);
                        // TODO: Try to find any existing ?dioxus= query params and remove them
                        const new_attribute_value = attribute_value_parts.join(url + random_query_param);
                        element.setAttribute(attribute, new_attribute_value);
                    }
                }
            }
        }
    };
    ws.onclose = reload_upon_connect;
})()