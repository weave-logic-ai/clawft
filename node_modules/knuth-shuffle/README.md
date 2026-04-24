knuth-shuffle
=============

The Fisher-Yates (aka Knuth) shuffle for Browser and Node.js

  * [Mike Bostock's Fisher–Yates Shuffle Visualization](http://bost.ocks.org/mike/shuffle/)
  * [How to randomize/shuffle a JavaScript array](http://stackoverflow.com/questions/2450954/how-to-randomize-shuffle-a-javascript-array)
  * [Fisher-Yates Shuffle on Wikipedia](http://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle)
  * [Doing the Microsoft Shuffle: Algorithm Fail in Browser Ballot](http://www.robweir.com/blog/2010/02/microsoft-random-browser-ballot.html)
  * [knuth-shuffle on NPM](https://npmjs.org/package/knuth-shuffle)

'nuf said.

The Fisher-Yates (Knuth) Shuffle
===

As Microsoft learned the hard way (see article below), `function random() { return 0.5 - Math.random() }` turns out to be no-so-random at all.

The fisher-yates shuffle is an algorithm so simple that even
[IEEE floating point math](http://blogs.adobe.com/bparadie/2011/11/22/0-2-0-1-0-30000000000000004/)
can't screw it up!

I put this on npm as `knuth-shuffle` because `fisher-yates-shuffle`
was just too long of a name and shuffle was already taken.

Browser Example
===

```html
<script src="https://raw.github.com/coolaj86/knuth-shuffle/master/index.js"></script>
```

```javascript
(function () {
  'use strict';

  var a = [2,11,37,42];
  var b;

  // The shuffle modifies the original array
  // calling a.slice(0) creates a copy, which is assigned to b
  b = window.knuthShuffle(a.slice(0));
  console.log(b);
}());
```

Node Example
===

Decentralized Install

```bash
npm install --save git+https://git.coolaj86.com/coolaj86/knuth-shuffle.js.git#v1.0
```

Centralized Install

```bash
npm install --save knuth-shuffle@1.0
```

```javascript
(function () {
  'use strict';

  var shuffle = require('knuth-shuffle').knuthShuffle;
  var a = [2,11,37,42];
  var b;

  // The shuffle modifies the original array
  // calling a.slice(0) creates a copy, which is assigned to b
  b = shuffle(a.slice(0));
  console.log(b);
}());
```
