(function () {
  'use strict';

  var shuffle = require('./').knuthShuffle
    , a = [2,11,37,42]
    , b
    ;

  // The shuffle modifies the original array
  // calling a.slice(0) creates a copy, which is assigned to b
  b = shuffle(a.slice(0));
  console.log(b);
}());
