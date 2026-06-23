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
    "s": "↓↓↓→→→→→→←←←←←←↑↑↑↑↑↑↑↑↓↓↓↓↓↓↓↓→→→→→→→←←←←←←←↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑→→→→←←↓↓→→→→↓↓→"
  },
  "remote_detonator": {
    "p": 1,
    "s": "^<<<<v"
  },
  "roach_motel": {
    "p": 1,
    "s": "^^"
  },
  "sacrifice": {
    "p": 1,
    "s": "<>"
  },
  "stampede": {
    "p": 1,
    "s": "^^^"
  },
  "web_lair": {
    "p": 1,
    "s": "^^vvvv"
  },
  "coop_world_v3": {
    "p": 2,
    "s": "↑↑ ↑→ ↑↑ ↑↑ ↑↑ ↑↑ ↑↑ ↑→ ↑↑ ↓→ ←→ ↑→ ↑→ ↑→ ↑→ ↑→ ↑→ →↓ ←. ↑↓ →← ↓← .→ .↑ ↑↑ ↑↑"
  },
  "cooperation": {
    "p": 2,
    "s": "↑↑ ↑→ ↑↑ ↑↑ ↑← ←↑ ↑→ ↑→ ↑→ ←→ ←↓ ←→ ←→ ←→ ←→ ↑→ →→ →→ ←→ ←← .← ←↓ ↓↓ ↓↓ ↓← ↓← ↓↓ ↓↓"
  },
  "tug_of_war": {
    "p": 2,
    "s": "^v vv >< vv vv vv vv <> <> <> <> <> >v << vv >> vv vv << >> << >> ^^ <^ ^^ >^ <v ^> ^< >^ >^ ^> ^^ ^< >< >< ^^ ^< ^> ^v ^^ ^^ ^^ ^^ ^^ >^ >^ ^^ ^^ v^ v^ <^ v^ v^ v^ v^ v^ v^ v^ v^ v^ v^ ^^ ^^ >^ >^ ^^ ^^ ^^ ^^ ^< ^< ^^ ^^ ^> ^> ^> ^> ^v ^v ^> ^> ^> ^^ ^^ ^> ^v ^v ^v ^v ^v ^v ^< ^< ^< ^< ^v ^v"
  },
  "cyborg_rats": {
    "p": 1,
    "s": "↑→→↑↑←←←→→→↑↑←←←→→→↑↑←←←→→→↑↑←←←"
  },
  "fakeout": {
    "p": 1,
    "s": "↑↑↓......→→←↓←↓................←←←→←←←←←→←→→←→↑→↑→↓←←↓↓↓↓↓↓↓↓↓↓→↑→↑↑→→→→→→→→→→"
  },
  "stalemate": {
    "p": 1,
    "s": "↑→→↑→↑→→↓↓←↓↓→←↑↑→↑↑→→→..............→←←←←←←←←←"
  },
  "unguided": {
    "p": 1,
    "s": "v<<<<<<<<<<<<vvvvvvvvvvvvvv>vv>><^<^^^^^^^^^^^^^^^>>>>>>>>>>>>>>>^vvvvvvvvvvvvvv>>vvvv<<<<^^<<<<<<^^>>>>>>"
  },
  "explosives": {
    "p": 1,
    "s": "↓↓↓←←↑↑↑↑←←↓↓↓↓↓↓→→→→↓↓←←←←←→→→→→↑↑←←←←↑↑↑↑↑↑↑↑→→→→→→→"
  },
  "explosives2": {
    "p": 1,
    "s": "↑←↓↓↓↓↓↓→→→→→←←←←←↑↑↑↑↑↑↑↑↓↓↓↓↓↓↓↓→←↑↓↑↑↑↑↑↑↑↑→→→→→→→"
  },
  "crushkill": {
    "p": 2,
    "s": "<> <> <> <> >^ ^v ^v ^< ^< ^<"
  },
  "platform": {
    "p": 1,
    "s": "vvv>vvv..^^.^^^^^^^^^<>>>>vvv"
  },
  "robotic_cheese": {
    "p": 2,
    "s": ">> ^> ^. ^. <. <^ ^^"
  },
  "guidance": {
    "p": 1,
    "s": "←←←←←←←←←←←↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓→→→→→→→→→←↑→→→→→←←←←←←←←←←←←←←↑↑↑↑↑↑←↓←↓→↑↑↑↑↑↑↑↑↑↑↑→→→→→↑↑↑↑↑↓↓←←←←←↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓→→→→→→→↑↑→→→→→→"
  },
  "intro": {
    "p": 1,
    "s": "↑↑↑→→→→↓↓↓←→↑↑↑←←←←↑↑↑↑↑↑←←↓↓↓↓↓←↓←↓↓↓↑↑↑→↑↑↑↑↑↑→→→↑→→"
  },
  "limited2": {
    "p": 1,
    "s": "→→→→→→→→→→→→→→←←←←←←←←←←←←←...←→.→→→→→→→→→→→→→←←←←←←←←←←←←←...←→....→→→→→→→→→→→→→←←←←←←←←←←←←←...←→→→→→→→←←←←←←←←←←←↑↑↑↑↑↑↑↑↑↑↑↑→→→→→→→→↑↑↑↑↑↑↑↑↑→→→→"
  },
  "lock_in": {
    "p": 1,
    "s": "↑↑→↓↓↓←↓↓←←↓←↓↓→↓←↑↑↑↑↑↓↓↓↓↓↓↑↑→→↑→→→→→↑↑↑↑↑↑↑↑↑↑↑↓←↓→←←←↑↑←←←←←←←←←←"
  },
  "more_rats": {
    "p": 1,
    "s": "↓←←→←→←→←→←→→↑↑↑↑↑↑←←→←→←→←←←"
  },
  "no_retreat": {
    "p": 1,
    "s": "↓←↑←←←←↑←→↑↑→→→↑↑←←↑←←←←→→→.←←←→←↑↑→→→→→→→→↑↑←←←←←←←←↑↑→→→→→→→→→→→→↑→→→→↑→→→"
  },
  "old_levels": {
    "p": 1,
    "s": "↓↓↓↓←←←←←"
  },
  "on_the_clock": {
    "p": 1,
    "s": "^>><vvvvv<v^^^^^>>v>>^<^^^^^^^>>>vvvvvvvvv^^^>>>><<>>vvv>>v>>>>>>vv<<<<<<<<<vv^^>>>>>>>>>^^<<<<<<^<^^^<<<vvv<<<<v<vvvv>>><<<<<<"
  },
  "order_of_operations": {
    "p": 1,
    "s": "<^v^^<^^^>^^<vv^vv^^^^^^^^^^>^v>vv<vvvvvvvv>>>>>^^^^^^^^^^^^^^^>>>>>vvvvvvv>vvvvvvv>>vv"
  },
  "overstep": {
    "p": 1,
    "s": "vvvvvv>>>>>>>>>v>><>><<^v^>>>^>^>^^^^^^^^^^^<<<<<>>>v>v>vvvvvvvvvvv<<<><<<<^<<<<^^<<^<<<^^<<^^^^^^>>>>>>>v>>>^^<<vvv<<>^^<<<<<vvv<<vvv>>vvvv<<<vv^^>>^^^^>^>>v^^^^^^^>>"
  },
  "order_of_operations_new_v2": {
    "p": 1,
    "s": "↓→→→→↑→↑↑↑→→↓→.....→→→↓←↓↓↓↓↓↓↓↓↓↓↓↓"
  },
  "planks": {
    "p": 1,
    "s": "→→→→←←←←←←←←←←↑↑↑↑↑↑←↓↓↓↓↓↓→→→→→→→→→→→→↑↑↑↑→↑↑↑↓↓↓←↓↓↓↓←←←←←←←←←←←←↑↑↑↑↑↑↑→→→↓→↓↓↓↓→→→→→→↑↑←↑↑↑←←←↓↓"
  },
  "rats": {
    "p": 1,
    "s": "↓←←↑↑↑→→→→←→↑"
  },
  "synchronicity": {
    "p": 1,
    "s": "→→↑↓→→↓↓↓←←↓↑↑↑↑←↓↑↓↑↓↑↑↑↓↓→↓↓↓→↓↓→→←←↑↑→→→→→→↑↑↑↑"
  },
  "tinderbox": {
    "p": 1,
    "s": "^>^^^^^<<<vv<<>.<>>>>^>v<<>>>v<v.><^v>><^^v<>^^<..v.^v^<v.>.vv^v<v..v^<^<<<<.vv^<^v>>"
  },
  "trapped_rat": {
    "p": 1,
    "s": "↓↓↓↑↓.→→←→→→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑↑↑↑↑←"
  },
  "trapped_rat2_v2": {
    "p": 1,
    "s": "←↑↑↑↓→←↑↓→←↑↓→←↑↑←←←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↑↑↑↑↑↑→"
  },
  "triggering_explosives_v3": {
    "p": 1,
    "s": "←↓↓↓↓←←↓↓↓↓←←↓↑→→↑↑↑↑→→↑↑↑↑→→↓↓↓↓→→↓↓↓↑↑↑←↑↑→←↑←↑←←↑↑↓↓←←↑←←↓↓↓→↑↑↑→→←→↓"
  },
  "triggers": {
    "p": 1,
    "s": "↓↓↓→→↑↑→→↓↓→→→→↑→↑↑←↑←←←↑↑→→→→↑↑←←←→→↑↑←↑↑←←↓↓↓←↓←↓↓↓↓←←←←←↑↑↑↑↑↑↑→→→↓↓"
  },
  "triggers2": {
    "p": 1,
    "s": "↑↑→↑→↑↑↑↓↓↓←↓←←←←←↑↑↑↑→←→←↓↓↓↓→→→→→↑→↑↑↑↑↑↓↓↓↓↓→→↑↑↑↑→→↓→↓→↓↓↓↓↓→←↑↑↑↑↑←↑←↑↑→←←←→←↓↓↓↓↓←←←↓←↓↓↓↓↓→→↓→→→→"
  },
  "webs": {
    "p": 1,
    "s": "←←↑↑→→→↑→↑↑←←↑→→→→→→→→→↓↓↓↓↓←→←→←→←←←↑↑"
  },
  "world": {
    "p": 2,
    "s": "^^ << <^ ^^ ^^ v^ v^ <^ <^ ^^ ^^ v^ v^ >^ >^ >^ >^ >v >^ ^^ ^^ v^ v^ v^ v^ >^ >^ <^ <^ ^^ ^^ >^ >^ ^^ ^^ v^ v^ <^ <^ <^ <^ ^^ ^^ ^^ ^^ <^ <^ <^ <^ >^ ^^ ^^ ^^ ^^ >^ >^ v^ v^ ^^ ^^ >^ >^ v^ v^ ^^ ^^ ^^ ^^ ^^ ^^ >^ >^ >^ v^ vv ^< ^< ^^ ^^ ^> ^^ ^^ ^v ^v ^< ^< ^^ ^^ ^v ^v ^< ^< ^^ ^^ ^v ^v ^v ^< ^< ^^ ^^ ^^"
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
