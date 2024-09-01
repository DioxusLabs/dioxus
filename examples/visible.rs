//! Port of the https://codepen.io/ryanfinni/pen/VwZeGxN example

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut animated_classes = use_signal(|| ["animated-text", ""]);

    rsx! {
        head::Link { rel: "stylesheet", href: asset!("./examples/assets/visible.css") }

        div {
            class: "container",

            p {
                "Scroll to the bottom of the page. The text will transition in when it becomes visible in the viewport."
            }

            p {
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Suspendisse vestibulum purus non porta
                ullamcorper. Vivamus tempus, massa in posuere tincidunt, lorem purus porta est, viverra tristique ipsum eros
                vitae metus. Suspendisse sagittis neque eget finibus gravida. Etiam sem urna, pulvinar eu mattis ut, sodales
                non massa. Morbi viverra luctus convallis. Curabitur ac massa mauris. Curabitur ut scelerisque nunc. Nulla
                condimentum porttitor est ac varius. Sed vel dui sed enim rutrum faucibus vitae eu eros. Curabitur eros leo,
                euismod ac ante eu, viverra malesuada diam."
            }

            p {
                "Nullam quis ipsum sagittis augue imperdiet fermentum. Morbi dapibus metus tempus, ullamcorper sem sit amet,
                dignissim felis. Curabitur arcu nulla, mattis hendrerit gravida at, sodales et lectus. Phasellus id porta
                quam. Sed in ex posuere, molestie mi eu, accumsan lectus. Cras erat massa, mollis vitae varius vel,
                hendrerit sit amet mi. Etiam nisl leo, sollicitudin non orci vel, lobortis consectetur metus. Sed cursus
                quam sapien, vehicula pharetra quam malesuada a. Curabitur in molestie arcu. Mauris nec leo venenatis,
                pulvinar lectus vel, convallis nibh. Aliquam a tellus eu metus hendrerit ultrices at blandit dolor. Praesent
                pharetra enim quis nunc bibendum, eu facilisis lacus auctor. Aliquam erat volutpat."
            }

            p {
                "Nam rhoncus vel erat et efficitur. Proin iaculis molestie erat, at sagittis enim finibus non. Aliquam tempus elit sit
                amet leo porta, a porttitor erat consequat. Praesent faucibus odio vitae purus pharetra aliquet. Fusce sit amet interdum
                ante. Sed tempor, purus quis porttitor ornare, quam purus dapibus neque, dapibus vulputate lorem ex nec elit. Maecenas
                in auctor mi, id sodales massa. Sed orci tellus, vestibulum et euismod iaculis, egestas vitae nulla. Nam a ornare ex, in
                semper nisl. Aliquam venenatis tortor arcu. Integer suscipit porta arcu."
            }

            p {
                "Aliquam eu nibh diam. Aliquam non cursus leo. Curabitur facilisis lacus ut nisi fringilla feugiat. Maecenas
                augue purus, pretium a magna non, auctor finibus justo. Fusce imperdiet libero ac quam elementum, vel
                vehicula nibh malesuada. Aliquam rhoncus quis nunc id aliquet. Duis ac finibus nisi. Donec fringilla tempor
                nibh ac tempor. Morbi a lectus vel tellus tincidunt posuere ut vel nisi. Proin aliquam ex libero, congue
                accumsan augue eleifend in. Nam sit amet dictum ipsum."
            }

            p {
                "Nulla eu ipsum ultricies, gravida elit quis, egestas urna. Orci varius natoque penatibus et magnis dis
                parturient montes, nascetur ridiculus mus. Proin blandit porta sem, eget lobortis sem pellentesque sit
                amet. Proin porttitor felis non tempus rhoncus. Cras lobortis accumsan nisi, at faucibus nibh laoreet
                quis. Vestibulum nisl urna, ullamcorper ut urna nec, venenatis consectetur massa. Aliquam erat
                volutpat. Maecenas facilisis diam eget consectetur elementum. Etiam magna mauris, vulputate quis gravida ut,
                tristique vel orci. Mauris condimentum nibh felis, ut feugiat dolor sodales vel. Pellentesque non iaculis
                sem, vel ullamcorper tellus. Nam dictum accumsan metus sed imperdiet. Phasellus malesuada nisl urna, quis
                consectetur nibh elementum ac. Nulla vitae dolor quis turpis pellentesque molestie a quis mauris. Integer
                enim leo, tincidunt vel leo sed, consequat finibus dolor. Phasellus elit libero, cursus non orci quis,
                suscipit rhoncus quam."
            }

            p {
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Suspendisse vestibulum purus non porta ullamcorper. Vivamus
                tempus, massa in posuere tincidunt, lorem purus porta est, viverra tristique ipsum eros vitae metus. Suspendisse
                sagittis neque eget finibus gravida. Etiam sem urna, pulvinar eu mattis ut, sodales non massa. Morbi viverra luctus
                convallis. Curabitur ac massa mauris. Curabitur ut scelerisque nunc. Nulla condimentum porttitor est ac varius. Sed vel
                dui sed enim rutrum faucibus vitae eu eros. Curabitur eros leo, euismod ac ante eu, viverra malesuada diam."
            }

            p {
                "Nullam quis ipsum sagittis augue imperdiet fermentum. Morbi dapibus metus tempus, ullamcorper sem sit amet,
                dignissim felis. Curabitur arcu nulla, mattis hendrerit gravida at, sodales et lectus. Phasellus id porta
                quam. Sed in ex posuere, molestie mi eu, accumsan lectus. Cras erat massa, mollis vitae varius vel,
                hendrerit sit amet mi. Etiam nisl leo, sollicitudin non orci vel, lobortis consectetur metus. Sed cursus
                quam sapien, vehicula pharetra quam malesuada a. Curabitur in molestie arcu. Mauris nec leo venenatis,
                pulvinar lectus vel, convallis nibh. Aliquam a tellus eu metus hendrerit ultrices at blandit dolor. Praesent
                pharetra enim quis nunc bibendum, eu facilisis lacus auctor. Aliquam erat volutpat."
            }

            p {
                "rhoncus vel erat et efficitur. Proin iaculis molestie erat, at sagittis enim finibus non. Aliquam tempus
                elit sit amet leo porta, a porttitor erat consequat. Praesent faucibus odio vitae purus pharetra
                aliquet. Fusce sit amet interdum ante. Sed tempor, purus quis porttitor ornare, quam purus dapibus neque,
                dapibus vulputate lorem ex nec elit. Maecenas in auctor mi, id sodales massa. Sed orci tellus, vestibulum et
                euismod iaculis, egestas vitae nulla. Nam a ornare ex, in semper nisl. Aliquam venenatis tortor
                arcu. Integer suscipit porta arcu."
            }

            h2 {
                class: animated_classes().join(" "),

                onvisible: move |evt| {
                    let data = evt.data();

                    if let Ok(is_intersecting) = data.is_intersecting() {
                        animated_classes.write()[1] = if is_intersecting { "visible" } else { "" };
                    }
                },

                "Animated Text"
            }
        }
    }
}
