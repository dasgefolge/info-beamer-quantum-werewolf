#![warn(trivial_casts)]
#![deny(unused_qualifications)]
#![forbid(unused, unused_import_braces)]

extern crate quantum_werewolf;
extern crate rand;

use std::{iter, thread};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use quantum_werewolf::{Handler, Player};
use quantum_werewolf::game::{self, Faction, Role};
use quantum_werewolf::game::state::Signups;

use rand::{Rng, thread_rng};

enum Text {
    ChooseHealTarget,
    ChooseInvestigationTarget,
    ChooseLynchTarget,
    ChooseWerewolfKillTarget,
    DuplicatePlayerName,
    RecvInvestigation,
    Signup,
    Winners
}

impl Text {
    fn as_str(&self) -> &str {
        use Text::*;

        match *self {
            ChooseHealTarget => "Wen möchtest du heilen?",
            ChooseInvestigationTarget => "Wessen Partei möchtest du erfahren?",
            ChooseLynchTarget => "Wen lyncht ihr?",
            ChooseWerewolfKillTarget => "Wen tötest du als Werwolf?",
            DuplicatePlayerName => "Dieser Name ist schon vergeben.",
            RecvInvestigation => "Ergebnis deiner Nachtaktion:",
            Signup => "Nächster Spielername (leer lassen wenn fertig):",
            Winners => "Das Spiel ist vorbei. Gewonnen haben:"
        }
    }
}

#[derive(Debug)]
enum Mode {
    Loading,
    AnnouncePlayers(String, Vec<String>),
    Choose(String, Vec<String>, usize),
    ChooseOptional(String, Vec<String>, Option<usize>),
    Deaths(Vec<(String, Role)>),
    Input(String, String, Result<(), String>),
    ShowFaction(String, Faction)
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Loading
    }
}

#[derive(Debug)]
struct PlayerData {
    handler: Rc<RefCell<IbHandler>>,
    name: String,
    new_id: Option<usize>
}

impl PartialEq for PlayerData {
    fn eq(&self, other: &PlayerData) -> bool {
        self.name == other.name
    }
}

impl Eq for PlayerData {}

impl Hash for PlayerData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IbPlayer(Rc<RefCell<PlayerData>>);

impl IbPlayer {
    fn new(handler: Rc<RefCell<IbHandler>>, name: String) -> IbPlayer {
        IbPlayer(Rc::new(RefCell::new(PlayerData {
            handler, name,
            new_id: None
        })))
    }

    fn name(&self) -> String {
        self.0.borrow().name.to_owned()
    }
}

impl Hash for IbPlayer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

impl Player for IbPlayer {
    fn recv_id(&self, new_id: usize) {
        self.0.borrow_mut().new_id = Some(new_id);
    }

    fn choose_heal_target(&self, possible_targets: Vec<&IbPlayer>) -> Option<IbPlayer> {
        let data = self.0.borrow();
        let mut handler = data.handler.borrow_mut();
        handler.show_to_player(&self.name());
        handler.choose_optional_player(Text::ChooseHealTarget, possible_targets)
    }

    fn choose_investigation_target(&self, possible_targets: Vec<&IbPlayer>) -> Option<IbPlayer> {
        let data = self.0.borrow();
        let mut handler = data.handler.borrow_mut();
        handler.show_to_player(&self.name());
        handler.choose_optional_player(Text::ChooseInvestigationTarget, possible_targets)
    }

    fn choose_werewolf_kill_target(&self, possible_targets: Vec<&IbPlayer>) -> IbPlayer {
        let data = self.0.borrow();
        let mut handler = data.handler.borrow_mut();
        handler.show_to_player(&self.name());
        handler.choose_player(Text::ChooseWerewolfKillTarget, possible_targets)
    }

    fn recv_exile(&self, reason: &str) {
        let data = self.0.borrow();
        let mut handler = data.handler.borrow_mut();
        handler.show_to_all();
        handler.announce_players(reason, iter::once(self)); //TODO add “player was exiled” text before reason
    }

    fn recv_investigation(&self, faction: Faction) {
        let data = self.0.borrow();
        let mut handler = data.handler.borrow_mut();
        handler.show_to_player(&self.name());
        handler.show_faction(Text::RecvInvestigation, faction);
    }
}

#[derive(Debug, Default)]
struct IbHandler {
    mode: Mode,
    pending_show_change: Option<Option<String>>
}

impl IbHandler {
    fn announce_players<'a, I: IntoIterator<Item = &'a IbPlayer>>(&mut self, text: &str, players: I) {
        self.mode = Mode::AnnouncePlayers(text.into(), players.into_iter().map(|player| player.name().to_owned()).collect::<BTreeSet<_>>().into_iter().collect());
        self.serialize();
    }

    fn choose_optional_player<'a, I: IntoIterator<Item = &'a IbPlayer>>(&mut self, text: Text, possible_targets: I) -> Option<IbPlayer> {
        let mut possible_targets = possible_targets.into_iter().collect::<Vec<_>>();
        thread_rng().shuffle(&mut possible_targets);
        self.mode = Mode::ChooseOptional(text.as_str().into(), possible_targets.iter().map(|player| player.name().to_owned()).collect(), None);
        self.serialize();
        unimplemented!(); //TODO read keyboard input
    }

    fn choose_player<'a, I: IntoIterator<Item = &'a IbPlayer>>(&mut self, text: Text, possible_targets: I) -> IbPlayer {
        let mut possible_targets = possible_targets.into_iter().collect::<Vec<_>>();
        thread_rng().shuffle(&mut possible_targets);
        self.mode = Mode::Choose(text.as_str().into(), possible_targets.iter().map(|player| player.name().to_owned()).collect(), 0);
        self.serialize();
        unimplemented!(); //TODO read keyboard input
    }

    fn get_input<F: FnOnce(&str) -> Result<(), String>>(&mut self, text: Text, validate: F) -> String {
        self.mode = Mode::Input(text.as_str().into(), String::default(), validate(""));
        self.serialize();
        unimplemented!(); //TODO read keyboard input
    }

    fn show_faction(&mut self, text: Text, faction: Faction) {
        self.mode = Mode::ShowFaction(text.as_str().into(), faction);
        self.serialize();
    }

    fn show_to_all(&mut self) {
        self.pending_show_change = Some(None);
        self.serialize();
    }

    fn show_to_player(&mut self, name: &str) {
        self.pending_show_change = Some(Some(name.into()));
        self.serialize();
    }

    fn serialize(&self) {
        unimplemented!(); //TODO
    }
}

impl Handler<IbPlayer> for IbHandler {
    fn choose_lynch_target(&mut self, possible_targets: HashSet<&IbPlayer>) -> Option<IbPlayer> {
        self.show_to_all();
        self.choose_optional_player(Text::ChooseLynchTarget, possible_targets)
    }

    fn announce_deaths<I: IntoIterator<Item = (IbPlayer, Role)>>(&mut self, deaths: I) {
        self.show_to_all();
        self.mode = Mode::Deaths(deaths.into_iter().map(|(player, role)| (player.name().to_owned(), role)).collect());
        self.serialize();
        unimplemented!(); //TODO wait for return key press
    }

    fn announce_probability_table<I: IntoIterator<Item = Result<(f64, f64, f64), Faction>>>(&mut self, _ /*probability_table*/: I) {
        self.show_to_all();
        unimplemented!(); //TODO
    }
}

impl Handler<IbPlayer> for Rc<RefCell<IbHandler>> {
    fn choose_lynch_target(&mut self, possible_targets: HashSet<&IbPlayer>) -> Option<IbPlayer> {
        self.borrow_mut().choose_lynch_target(possible_targets)
    }

    fn announce_deaths<I: IntoIterator<Item = (IbPlayer, Role)>>(&mut self, deaths: I) {
        self.borrow_mut().announce_deaths(deaths);
    }

    fn announce_probability_table<I: IntoIterator<Item = Result<(f64, f64, f64), Faction>>>(&mut self, probability_table: I) {
        self.borrow_mut().announce_probability_table(probability_table);
    }

    fn cannot_lynch(&mut self, target: IbPlayer) {
        self.borrow_mut().cannot_lynch(target);
    }
}

fn main() {
    let handler = Rc::new(RefCell::new(IbHandler::default()));
    let mut game_state = Signups::default();
    loop {
        let handler_clone = handler.clone();
        let name = handler_clone.borrow_mut().get_input(Text::Signup, |s| if s.is_empty() || !game_state.is_signed_up(&IbPlayer::new(handler.clone(), s.into())) {
            Ok(())
        } else {
            Err(Text::DuplicatePlayerName.as_str().into())
        });
        if name.is_empty() {
            break;
        }
        if !game_state.sign_up(IbPlayer::new(handler.clone(), name)) {
            println!("[ !! ] duplicate player name"); //TODO integrate duplicate check into name input mode
        }
    }
    let winners = game::run(handler.clone(), game_state).expect("failed to start game");
    handler.borrow_mut().announce_players(Text::Winners.as_str(), winners.iter());
    // keep info-beamer running
    loop { thread::park(); }
}
