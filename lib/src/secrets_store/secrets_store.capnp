@0x89ae7248ac2e8067;

using Api = import "../api/api.capnp";

enum KeyDerivationType {
    argon2 @0;
}

# Enumeration of all cipher suites
enum KeyType {
    rsaAesGcm @0;
    ed25519Chacha20Poly1305 @1;
}


# Layout of a (private ring block)
struct Ring {
    id @0 : Text;
    name @1 : Text;
    email @2 : Text;
    publicKeys @3 : List(PublicKey);
    privateKeys @4 : List(PrivateKey);
    hidden @5: Bool = false;

    struct PublicKey {
        type @0 : KeyType;
        key @1 : Data;
    }

    struct PrivateKey {
        type @0 : KeyType;
        derivationType @1: KeyDerivationType;
        preset @2 : UInt8;
        nonce @3 : Data;
        cryptedKey @4 : Data;
    }
}

# Layout of a data block
# Since "Data" is reserved by Cap'n Proto, we just call it Block
struct Block {
    headers @0 : List(Header);
    content @1 : Data;

    struct Header {
        type @0 : KeyType;
        commonKey @1 : Data;
        recipients @2 : List(RecipientKey);
    }

    struct RecipientKey {
        id @0 : Text;
        cryptedKey @1: Data;
    }
}

# Internal layout of an index
# This is supposed to be in-(secure)-memory only. When written to some persistent medium
# an index has to be wrapped in a block like all other secrets
struct Index {
    heads @0 : List(Head);
    entries @1 : List(Entry);

    enum HeadOperation {
        add @0;
        delete @1;
    }

    struct Head {
        nodeId @0: Text;
        operation @1 : HeadOperation;
        blockId @2 : Text;
    }

    struct Entry {
        entry @0 : Api.SecretEntry;
        versionRefs @1 : List(Api.Secret.VersionRef);
    }
}