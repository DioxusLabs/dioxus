fn a() -> Element {
    rsx! {
        <table>
            {
                items
                    .iter()
                    .map(|x| {
                        let y = f(x);
                        rsx! {
                            <tr>
                                <td>"one"</td>
                                <td>"two"</td>
                                <td>
                                    <a href="x">"four"</a>
                                </td>
                            </tr>
                        }
                    })
            }
        </table>
    }
}
