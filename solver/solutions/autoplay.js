/**
 * Infestation auto-player — verified solutions (oracle-confirmed result=Won).
 * Paste into the dev console at https://davidspies.github.io/infestation/
 *   infestation.list()          list solved levels
 *   infestation.play("rats")    auto-play a level (navigate to it first)
 *   infestation.stop()          stop;  infestation.speed = 150  (ms/move)
 * Two-player levels drive P1 with arrow keys and P2 with WASD.
 */
(function () {
  const SOL = {
  "blackhole_v2": {
    "p": 1,
    "s": "vvv>>>>>><<<<<<^^^^^^^^vvvvvvvv>>>>>>><<<<<<<^^^^^^^^^^^^^^^^^>>>><<vv>>>>vv>"
  },
  "chase": {
    "p": 1,
    "s": "^>>>v^^^>^^>>>>>v>>vvvvv^^^^^^<<<<v<<<v^<^<^<<>>>>v>>>>>vvvv^^^^<<<^<^<<^^^^^^^^<<<<<^^vvvvvvvvvvvvv>>>>>>>>vv^^<<vvvvv<>>>>>>>>>>>>>>^^v<>^^^^^^^^^^^^^^^^^^<<<<>>vvvv<vvvvvvvvvvvv<vvv<<<<<<<<<<<<<<<"
  },
  "coop_world_v3": {
    "p": 2,
    "s": "^^ ^> ^^ ^^ ^^ ^^ ^^ ^> ^^ v> <> ^> ^> ^> ^> ^> ^> >v <. ^v >< v< .> .^ ^^ ^^"
  },
  "cooperation": {
    "p": 2,
    "s": "^^ ^> ^^ ^^ ^< <^ ^> ^> ^> <> <v <> <> <> <> ^> >> >> <> << .< <v vv vv v< v< vv vv"
  },
  "cyborg_rats": {
    "p": 1,
    "s": "^>>^^<<<>>>^^<<<>>>^^<<<>>>^^<<<"
  },
  "fakeout": {
    "p": 1,
    "s": "^^v......>><v<v................<<<><<<<<><>><>^>^>v<<vvvvvvvvvv>^>^^>>>>>>>>>>"
  },
  "stalemate": {
    "p": 1,
    "s": "^>>^>^>>vv<vv><^^>^^>>>..............><<<<<<<<<"
  },
  "unguided": {
    "p": 1,
    "s": "^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^vv>>>>vvvvv>^^^^^<<<<<<<<<<<<<<<vvvvvvvvvvvvvvv>>>>>v<<>^>>>^^>>>>>>"
  },
  "explosives": {
    "p": 1,
    "s": "vvv<<^^^^<<vvvvvv>>>>vv<<<<<>>>>>^^<<<<^^^^^^^^>>>>>>>"
  },
  "explosives2": {
    "p": 1,
    "s": "^<vvvvvv>>>>><<<<<^^^^^^^^vvvvvvvv><^v^^^^^^^^>>>>>>>"
  },
  "guidance": {
    "p": 1,
    "s": "<<<<<<<<<<<vvvvvvvvvvvvvvvv>>>>>>>>><^>>>>><<<<<<<<<<<<<<^^^^^^<v<v>^^^^^^^^^^^>>>>>^^^^^vv<<<<<vvvvvvvvvvvvvvv>>>>>>>^^>>>>>>"
  },
  "limited2": {
    "p": 1,
    "s": ">>>>>>>>>>>>>><<<<<<<<<<<<<...<>.>>>>>>>>>>>>><<<<<<<<<<<<<...<>....>>>>>>>>>>>>><<<<<<<<<<<<<...<>>>>>>><<<<<<<<<<<^^^^^^^^^^^^>>>>>>>>^^^^^^^^^>>>>"
  },
  "lock_in": {
    "p": 1,
    "s": "^^>vvv<vv<<v<vv>v<^^^^^vvvvvv^^>>^>>>>>^^^^^^^^^^^v<v><<<^^<<<<<<<<<<"
  },
  "more_rats": {
    "p": 1,
    "s": "v<<><><><><>>^^^^^^<<><><><<<"
  },
  "no_retreat": {
    "p": 1,
    "s": "v<^<<<<^<>^^>>>^^<<^<<<<>>>.<<<><^^>>>>>>>>^^<<<<<<<<^^>>>>>>>>>>>>^>>>>^>>>"
  },
  "old_levels": {
    "p": 1,
    "s": "vvvv<<<<<"
  },
  "order_of_operations": {
    "p": 1,
    "s": "<^v^^<^^^>^^<vv^vv^^^^^^^^^^>^v>vv<vvvvvvvv>>>>>^^^^^^^^^^^^^^^>>>>>vvvvvvv>vvvvvvv>>vv"
  },
  "order_of_operations_new_v2": {
    "p": 1,
    "s": "v>>>>^>^^^>>v>.....>>>v<vvvvvvvvvvvv"
  },
  "planks": {
    "p": 1,
    "s": ">>>><<<<<<<<<<^^^^^^<vvvvvv>>>>>>>>>>>>^^^^>^^^vvv<vvvv<<<<<<<<<<<<^^^^^^^>>>v>vvvv>>>>>>^^<^^^<<<vv"
  },
  "rats": {
    "p": 1,
    "s": "v<<^^^>>>><>^"
  },
  "synchronicity": {
    "p": 1,
    "s": ">>^v>>vvv<<v^^^^<v^v^v^^^vv>vvv>vv>><<^^>>>>>>^^^^"
  },
  "tinderbox_v2": {
    "p": 1,
    "s": "^>^^^^^<<<vv<<>.<>>>>^>v<<>>>v<v.><^v>><^^v<>^^<..v.^v^<v.>.vv^v<v..v^<^<<<<.vv^<^v>>"
  },
  "trapped_rat": {
    "p": 1,
    "s": "vvv^v.>><>>>>^^^^^^<vvvvvv<<>>^^^^^^<vvvvvv<<>>^^^^^^<vvvvvv<<>>^^^^^^^^^^<"
  },
  "trapped_rat2_v2": {
    "p": 1,
    "s": "<^^^v><^v><^v><^^<<<vvvvvv<^^^^^^><><vvvvvv<^^^^^^><><vvvvvv<^^^^^^><><^^^^^^>"
  },
  "triggering_explosives_v3": {
    "p": 1,
    "s": "<vvvv<<vvvv<<v^>>^^^^>>^^^^>>vvvv>>vvv^^^<^^><^<^<<^^vv<<^<<vvv>^^^>><>v"
  },
  "triggers": {
    "p": 1,
    "s": "vvv>>^^>>vv>>>>^>^^<^<<<^^>>>>^^<<<>>^^<^^<<vvv<v<vvvv<<<<<^^^^^^^>>>vv"
  },
  "triggers2": {
    "p": 1,
    "s": "^^>^>^^^vvv<v<<<<<^^^^><><vvvv>>>>>^>^^^^^vvvvv>>^^^^>>v>v>vvvvv><^^^^^<^<^^><<<><vvvvv<<<v<vvvvv>>v>>>>"
  },
  "webs": {
    "p": 1,
    "s": "<<^^>>>^>^^<<^>>>>>>>>>vvvvv<><><><<<^^"
  },
  "world": {
    "p": 2,
    "s": "^^ ^< ^> ^< ^< ^< ^v ^< ^> <^ <^ v^ v^ v^ v^ v^ v^ <> <v v^ v^ v^ ^^ ^v ^< >> ^< ^< ^^ ^< ^< ^v >^ >< ^< ^< <v ^< ^^ v^ v^ >^ v^ v^ >^ v^ v^ v^ v^ v^ v^ v^ v^ v^ ^^ ^^ ^< >> >< >^ >v v< v< v^ <^ >< ^^ ^> ^v << << << <v ^> ^> ^< ^v ^> ^> ^^ ^< >> >< ^< ^^ v< v^ v^ v^ >^ >^ <v << ^< ^> >v >v ^^ ^^ vv vv <^ <v <^ <^ <> << << <^ ^> ^^ v^ v> >> >^ >> >> ^^ ^< ^< ^< << << << <^ >v ^^ ^^ ^v ^v ^^ ^v ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ v< v> v^ >v >v ^< ^< ^^ v< v> >^ >< ^< ^^ v< v> >^ >< ^^ ^^ v< v> << v^ v< v< v< v< v< >v >. v> v^ ^v ^v <^ <^ v^ v^ v^ v^ <^ v^ ^^ vv ^> <^ << v< v^"
  },
  "platform": {
    "p": 1,
    "s": "vvv>vvv..^^.^^^^^^^^^<>>>>vvv"
  },
  "robotic_cheese": {
    "p": 2,
    "s": ">> ^> ^. ^. <. <^ ^^"
  },
  "sacrifice": {
    "p": 1,
    "s": "<>"
  },
  "roach_motel": {
    "p": 1,
    "s": "^^"
  },
  "stampede": {
    "p": 1,
    "s": "^^^"
  },
  "remote_detonator": {
    "p": 1,
    "s": "^<<<<v"
  },
  "web_lair": {
    "p": 1,
    "s": "^^vvvv"
  }
};
  const P1 = { '^':'ArrowUp','v':'ArrowDown','<':'ArrowLeft','>':'ArrowRight' };
  const P2 = { '^':'w','v':'s','<':'a','>':'d' };
  const KC = { ArrowUp:38, ArrowDown:40, ArrowLeft:37, ArrowRight:39,
               w:87, a:65, s:83, d:68, ' ':32 };
  let stop=false, timer=null;
  function key(k){
    if(!k) return;
    const kc = KC[k]||0;
    for(const t of ['keydown','keypress','keyup']){
      const e=new KeyboardEvent(t,{key:k,code:k,keyCode:kc,which:kc,bubbles:true});
      document.dispatchEvent(e);
      const c=document.querySelector('canvas'); if(c) c.dispatchEvent(e);
    }
  }
  window.infestation = {
    speed: 200,
    list(){ console.log('Verified solutions:'); for(const n in SOL)
      console.log('  infestation.play("'+n+'")  ('+SOL[n].p+'p, '+
        (SOL[n].p==1?SOL[n].s.length:SOL[n].s.split(' ').length)+' moves)'); },
    play(name){
      const e = SOL[name];
      if(!e){ console.warn('unknown level: '+name); return; }
      const turns = e.p==1 ? e.s.split('') : e.s.split(' ');
      console.log('Playing '+name+' ('+turns.length+' turns @ '+this.speed+'ms)…');
      stop=false; if(timer) clearInterval(timer); let i=0;
      timer=setInterval(()=>{
        if(stop||i>=turns.length){ clearInterval(timer); if(!stop) console.log('✓ done '+name); return; }
        const t=turns[i++];
        if(e.p==1){ key(P1[t]); }
        else { const a=t[0], b=t[1]; key(P1[a]); key(b==='.'?' ':P2[b]); }
      }, this.speed);
    },
    stop(){ stop=true; if(timer) clearInterval(timer); console.log('stopped'); }
  };
  infestation.list();
})();
