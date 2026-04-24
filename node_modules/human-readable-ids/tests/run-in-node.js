/*jshint -W054 */
;(function (exports) {
  'use strict';

  var hri = exports.humanReadableIds || require('../index').hri
    , i
    ;

  for (i = 0; i < 100; i += 1) {
    console.log(hri.random());
  }
}('undefined' !== typeof exports && exports || new Function('return this')()));
