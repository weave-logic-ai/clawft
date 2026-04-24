human-readable-ids-js
=====================

Use JavaScript to generate human-readable ids from a lists of nouns and adjectives.

Generate ids in the format of `adjective-noun-#` such as

* silly-goose-37
* quick-cobra-15
* tricky-chicken-23
* brave-ladybug-90

My problem is that I often want ids that I can type without having to
look at twice (nor telling someone else twice).

I should be able to shout one of these ids across the room to a co-worker
or spouse and have them be able to enter it without any confusion.

Currently the id space is about 1,000,000 ids (100 * 100 * 100).

The goal is to have several billion possible combinations by adding
more words as well as expanding the ids to have verbs and adverbs.

For a larger address space now, consider:

  * Human Readable IDs for Node.js and Browser: <https://github.com/linus/greg/>
  * Human Readable IDs for Java: <https://github.com/PerWiklander/IdentifierSentence>
  * Human Readable IDs for Python: <https://gist.github.com/4447660>

All of these also have the benefit of bi-directional conversion, but not all of them
have words which are easy to pronounce and spell.

Usage
=======

### npm

```bash
# from npm
npm install --save human-readable-ids
```

```bash
# directly from git
npm install --save https://git.coolaj86.com/coolaj86/human-readable-ids.js.git
```

```javascript
'use strict';

var hri = require('human-readable-ids').hri;
var i;

// generate 100 random ids
for (i = 0; i < 100; i += 1) {
  console.log(hri.random());
}
```

### bower / browser

```
bower install --save human-readable-ids
```

```html
    <script src="bower_components/knuth-shuffle/index.js"></script>
    <script src="bower_components/human-readable-ids/assets/animals.js"></script>
    <script src="bower_components/human-readable-ids/assets/adjectives.js"></script>
    <script src="bower_components/human-readable-ids/index.js"></script>
```

```javascript
;(function (exports) {
  'use strict';

  var hri = exports.humanReadableIds || require('human-readable-ids').hri;
  var i;

  for (i = 0; i < 100; i += 1) {
    console.log(hri.random());
  }
}('undefined' !== typeof exports && exports || new Function('return this')()));
```

Contributing
============

Please add more words that fit these criteria

  * Easy-to-spell - such that even my redneck dad can manage
  * Phonetically distinct - no words like bore/boar
  * Few syllables
  * Well-known

All of the words are sorted alphabetically (`sort -u`) and stored in
the `*.txt` files.
The pre-publish script outputs the formatted javascript.

### Bad Examples

  * wednesday, hamster, ostrich (difficult to spell)
  * grey, gray, bore, boar (two ways of spelling the same word or sound)
  * prawn (not well-known)

Resources
=========

Add more words and strategies from

  * <http://blog.asana.com/2011/09/6-sad-squid-snuggle-softly/>
  * The Dolch List <http://www.mrsperkins.com/dolch-words-all.html>
  * <http://simple.wikipedia.org/wiki/Wikipedia:Basic_English_alphabetical_wordlist>
  * <https://github.com/zacharyvoase/humanhash>
  * <https://gist.github.com/ucnv/1121015>
  * <https://gist.github.com/vikhyat/105610>
  * <http://simple.wikipedia.org/wiki/Wikipedia:List_of_1000_basic_words>
  * <http://grammar.yourdictionary.com/parts-of-speech/adverbs/list-of-100-adverbs.html>

TODO
====

Acheive 1 trillion ids with a combination such as

`number adjective noun verb adverb`

`42-red-foxes-run-quickly`

And allow choosing various formats based on the desired
number of ids.

  * `adjective noun number`
  * `number adjective noun verb`
