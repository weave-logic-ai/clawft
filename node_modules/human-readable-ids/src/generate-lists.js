'use strict';

var fs = require('fs')
  , path = require('path')
  , files = {}
  , master = {}
  , tplFile
  ;

tplFile = fs.readFileSync(path.join(__dirname, '..', 'src', 'tpl.js'), 'utf8');

['animals', 'adjectives'].forEach(function (key) {
  files[key] = path.join(__dirname, '..', 'src', key + '.txt');
  master[key] = fs.readFileSync(files[key], 'utf8').trim().split('\n');

  fs.writeFileSync(
    path.join(__dirname, '..', 'assets', key + '.js')
  , tplFile.replace(/{{\s*setname\s*}}/, key).replace(/{{\s*set\s*}}/, JSON.stringify(master[key]))
  , 'utf8'
  );
});
