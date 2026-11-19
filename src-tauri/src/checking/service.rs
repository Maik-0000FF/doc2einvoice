// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The validator on a thread of its own.
//!
//! A validator is bound to the thread it is used on and keeps what it has
//! compiled. It therefore lives on one thread for as long as the application
//! runs; whoever wants an invoice checked sends it there and waits for the
//! answer.

use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use doc2einvoice_validation::{Profile, Validator, specification_identifier};

use super::document::Document;
use super::outcome::{InvoiceCheck, Refusal};

/// Name of the thread, as debuggers and crash reports show it.
const THREAD_NAME: &str = "validation";

/// What the validation thread is asked to do.
enum Request {
    /// Check a document as the profile it suggests, and keep it.
    Check(Document),
    /// Check the document kept from the last request as the given profile.
    CheckAgain(Profile),
}

struct Job {
    request: Request,
    reply: Sender<Result<InvoiceCheck, Refusal>>,
}

/// The way to the validation thread. Clones lead to the same thread; it ends
/// when the last of them is dropped.
#[derive(Debug, Clone)]
pub struct ValidationService {
    jobs: Sender<Job>,
}

impl ValidationService {
    /// Starts the validation thread.
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system if the thread cannot be
    /// started.
    pub fn start() -> io::Result<Self> {
        let (jobs, received) = mpsc::channel();
        thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || serve(&received))?;
        Ok(Self { jobs })
    }

    /// Checks a document as the profile its specification identifier suggests.
    ///
    /// # Errors
    ///
    /// Returns the [`Refusal`] that says why the document was not checked.
    pub fn check(&self, document: Document) -> Result<InvoiceCheck, Refusal> {
        self.ask(Request::Check(document))
    }

    /// Checks the document of the last successful check as another profile.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::NothingToCheck`] if no document was checked before,
    /// and otherwise the [`Refusal`] that says why it was not checked.
    pub fn check_again(&self, profile: Profile) -> Result<InvoiceCheck, Refusal> {
        self.ask(Request::CheckAgain(profile))
    }

    /// Sends a request and waits for its answer. A thread that is gone, which
    /// only a failure inside it can cause, is an internal failure.
    fn ask(&self, request: Request) -> Result<InvoiceCheck, Refusal> {
        let (reply, answer) = mpsc::channel();
        self.jobs
            .send(Job { request, reply })
            .map_err(|_| Refusal::Internal)?;
        answer.recv().map_err(|_| Refusal::Internal)?
    }
}

/// The validation thread: answers one request after the other until nobody
/// can send any more.
fn serve(jobs: &Receiver<Job>) {
    let mut validator = Validator::new();
    let mut kept: Option<Document> = None;

    for job in jobs {
        let answer = match job.request {
            Request::Check(document) => {
                let answer = check(&mut validator, &document, None);
                // Only what was checked is kept: a file that is no invoice
                // does not replace the invoice checked before.
                if answer.is_ok() {
                    kept = Some(document);
                }
                answer
            }
            Request::CheckAgain(profile) => kept
                .as_ref()
                .ok_or(Refusal::NothingToCheck)
                .and_then(|document| check(&mut validator, document, Some(profile))),
        };
        // Whoever asked may have stopped waiting; then the answer is of no use.
        let _ = job.reply.send(answer);
    }
}

/// Checks a document as the given profile, or as the one it suggests.
fn check(
    validator: &mut Validator,
    document: &Document,
    profile: Option<Profile>,
) -> Result<InvoiceCheck, Refusal> {
    let identifier = specification_identifier(&document.text)?;
    let suggested = Profile::suggested_for(identifier.as_deref());
    let report = validator.validate(&document.text, profile.unwrap_or(suggested))?;

    Ok(InvoiceCheck::new(document, identifier, suggested, &report))
}
