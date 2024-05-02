function FunDropDown(props) {
  return <Dropdown show={props.visible}>
    A dropdown list
    <Menu
      title="Menu Title"
      icon={props.menu.icon}
      onHide={(e) => console.log(e)}
      onShow={(e) => console.log(e)}
    >
      <MenuItem>Do Something</MenuItem>
      {
        shouldDoSomethingFun()
          ? <MenuItem>Do Something Fun!</MenuItem>
          : <MenuItem>Do Something Else</MenuItem>
      }
    </Menu>
  </Dropdown>;
}


