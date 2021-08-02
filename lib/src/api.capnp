@0x981c355b6da046c4; 

struct Option(T) {
    union {
        some @0 : T;
        none @1 : Void;
    }
}

struct StoreConfig {
    name @0 : Text;
    storeUrl @1 : Text;
    clientId @2 : Text;
    autolockTimeoutSecs @3 : UInt64;
    defaultIdentityId @4 : Option(Text) = (none = void);
}

struct EventData {
    union {
        storeUnlocked :group {
            storeName @0 : Text;
            identity @1 : Identity;
        }
        storeLocked :group {
            storeName @2 : Text;
        }
        secretOpened :group {
            storeName @3 : Text;
            identity @4 : Identity;
            secretId @5 : Text;
        }
        secretVersionAdded :group {
            storeName @6 : Text;
            identity @7 : Identity;
            secretId @8 : Text;
        }
        identityAdded :group {
            storeName @9 : Text;
            identity @10 : Identity;
        }
        clipboardProviding :group {
            storeName @11 : Text;
            blockId @12 : Text;
            property @13 : Text;
        }
        clipboardDone @14 : Void;
    }
}

struct Event {
    id @0 : UInt64;
    data @1 : EventData;
}

struct PasswordGeneratorParam {
    union {
        chars @0: PasswordGeneratorCharsParam;
        words @1: PasswordGeneratorWordsParam;
    }

    struct PasswordGeneratorCharsParam {
        numChars @0: UInt8;
        includeUppers @1: Bool;
        includeNumbers @2: Bool;
        includeSymbols @3: Bool;
        requireUpper @4: Bool;
        requireNumber @5: Bool;
        requireSymbol @6: Bool;
        exlcudeSimilar @7: Bool;
        excludeAmbiguous @8: Bool;
   }

    struct PasswordGeneratorWordsParam {
        numWords @0: UInt8;
        delim @1: UInt32;
    }
}

struct Identity {
    id @0 : Text;
    name @1 : Text;
    email @2: Text;
    hidden @3: Bool = false;
}

struct Status {
    locked @0 : Bool;
    unlockedBy @1: Option(Identity);
    autolockAt @2 : Int64;
    version @3 : Text;
    autolockTimeout @4: UInt64;
}

enum SecretType {
    login @0;
    note @1;
    licence @2;
    wlan @3;
    password @4;
    other @5;
}

struct SecretEntry {
    id @0 : Text;
    timestamp @1 : Int64;
    name @2 : Text;
    type @3 : SecretType;
    tags @4 : List(Text);
    urls @5 : List(Text);
    deleted @6 : Bool;
}

struct SecretListFilter {
    url @0 : Option(Text);
    tag @1 : Option(Text);
    type @2 : OptionType;
    name @3 : Option(Text);
    deleted @4 : Bool;

    # Workaround since enum can not be used as generic parameters
    struct OptionType {
        union {
            some @0 : SecretType;
            none @1 : Void;
        }
    }
}

struct SecretEntryMatch {
    entry @0 : SecretEntry;
    nameScore @1 : Int64;
    nameHighlights @2 : List(UInt64);
    urlHighlights @3 : List(UInt64);
    tagsHighlights @4 : List(UInt64);
}

struct SecretList {
    allTags @0 : List(Text);
    entries @1 : List(SecretEntryMatch);
}

struct SecretVersion {
    secretId @0 : Text;
    type @1 : SecretType;
    timestamp @2 : Int64;
    name @3 : Text;
    tags @4 : List(Text);
    urls @5 : List(Text);
    properties @6 : List(Property);
    attachments @7 : List(Attachment);
    deleted @8 : Bool;
    recipients @9 : List(Text);

    struct Property {
        key @0 : Text;
        value @1 : Text;
    }

    struct Attachment {
        name @0 : Text;
        mimeType @1 : Text;
        content @2 : Data;
    }
}

struct PasswordStrength {
    entropy @0 : Float64;
    crackTime @1 : Float64;
    crackTimeDisplay @2 : Text;
    score @3 : UInt8;
}


struct Secret {
    id @0 : Text;
    type @1 : SecretType;
    current @2 : SecretVersion;
    currentBlockId @3 : Text;
    versions @4 : List(VersionRef);
    passwordStrengths @5 : List(Estimate);

    struct Estimate {
        key @0 : Text;
        strength @1 : PasswordStrength;
    }

    struct VersionRef {
        blockId @0 : Text;
        timestamp @1 : Int64;
    }
}

struct Command {
    union {
        listStores @0 : Void;
        upsertStoreConfig @1 : StoreConfig;
        deleteStoreConfig @2 : Text;
        getDefaultStore @3 : Void;
        setDefaultStore @4 : Text;
        generateId @5 : Void;
        generatePassword @6 : PasswordGeneratorParam;
        pollEvents @7 : UInt64;

        status @8 : Text;
        lock @9 : Text;
        unlock :group {
            storeName @10 : Text;
            identityId @11 : Text;
            passphrase @12 : Data;
        }
        identities @13 : Text;
        addIdentity :group {
            storeName @14 : Text;
            identity @15 : Identity;
            passphrase @16 : Data;
        }
        changePassphrase :group {
            storeName @17 : Text;
            passphrase @18 : Data;
        }
        list :group {
            storeName @19 : Text;
            filter @20 : SecretListFilter;
        }
        updateIndex @21 : Text;
        add :group {
            storeName @22 : Text;
            secretVersion @23 : SecretVersion;
        }
        get :group {
            storeName @24 : Text;
            secretId @25 : Text;
        }
        getVersion :group {
            storeName @26 : Text;
            blockId @27 : Text;
        }

        secretToClipboard :group {
            storeName @28 : Text;
            blockId @29 : Text;
            properties @30 : List(Text);
            displayName @31 : Text;
        }

        clipboardIsDone @32 : Void;
        clipboardCurrentlyProviding @33 : Void;
        clipboardProvideNext @34 : Void;
        clipboardDestroy @35 : Void;
    }
}

struct ResultStoreConfigs {
    configs @0 : List(StoreConfig);
}

struct ResultOptionString {
    content @0 : Option(Text);
}

struct ResultIdentities {
    identities @0 : List(Identity);
}

struct ResultEvents {
    events @0 : List(Event);
}