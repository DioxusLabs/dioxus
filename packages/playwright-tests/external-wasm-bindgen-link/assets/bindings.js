export class Foo {
  constructor() {
    this.div = document.createElement('div');
    this.div.textContent = 'Hello from Foo';
    document.querySelector('#main').appendChild(this.div);
  }
}
