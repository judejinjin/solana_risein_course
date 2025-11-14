import dynamic from "next/dynamic";
import ReviewCard from "@/components/ReviewCard";
import { useEffect, useState } from "react";
import { Review } from "@/models/Review";
import * as web3 from "@solana/web3.js";
import { fetchReviews } from "@/util/fetchReviews";
import { useWallet } from "@solana/wallet-adapter-react";
import ReviewForm from "@/components/Form";

const AppBar = dynamic(() => import("@/components/AppBar").then((mod) => mod.AppBar), {
    ssr: false,
});

const REVIEW_PROGRAM_ID = "BvZjMnhfYFZsX55W27jSB9HcoD2QTevpBKBE3RE5pgw";

export default function Home() {
    const connection = new web3.Connection(web3.clusterApiUrl("devnet"));
    const { publicKey, sendTransaction } = useWallet();
    const [txid, setTxid] = useState("");

    const [reviews, setReviews] = useState<Review[]>([]);

    const [title, setTitle] = useState("");
    const [rating, setRating] = useState(0);
    const [description, setDescription] = useState("");

    useEffect(() => {
        const fetchAccounts = async () => {
            await fetchReviews(REVIEW_PROGRAM_ID, connection).then(setReviews);
        };
        fetchAccounts();
    }, []);

    const handleSubmit = () => {
        if (!title || !description) {
            alert("Please fill in all fields");
            return;
        }
        if (rating < 1 || rating > 5) {
            alert("Rating must be between 1 and 5");
            return;
        }
        const review = new Review(title, rating, description);
        handleTransactionSubmit(review);
    };

    const handleTransactionSubmit = async (review: Review) => {
        if (!publicKey) {
            alert("Please connect your wallet!");
            return;
        }

        console.log("=== Starting Transaction ===");
        console.log("Review data:", { title: review.title, rating: review.rating, description: review.description });
        console.log("Wallet publicKey:", publicKey.toBase58());
        console.log("Program ID:", REVIEW_PROGRAM_ID);

        try {
            const buffer = review.serialize();
            console.log("Serialized buffer length:", buffer.length);
            console.log("Serialized buffer:", buffer);

            const [pda] = await web3.PublicKey.findProgramAddressSync(
                [publicKey.toBuffer(), Buffer.from(review.title)],
                new web3.PublicKey(REVIEW_PROGRAM_ID)
            );
            console.log("PDA address:", pda.toBase58());

            // Check wallet balance
            const balance = await connection.getBalance(publicKey);
            console.log("Wallet balance:", balance / web3.LAMPORTS_PER_SOL, "SOL");

            if (balance === 0) {
                alert("Your wallet has no SOL. Please get some from https://faucet.solana.com");
                return;
            }

            const instruction = new web3.TransactionInstruction({
                keys: [
                    {
                        pubkey: publicKey,
                        isSigner: true,
                        isWritable: false,
                    },
                    {
                        pubkey: pda,
                        isSigner: false,
                        isWritable: true,
                    },
                    {
                        pubkey: web3.SystemProgram.programId,
                        isSigner: false,
                        isWritable: false,
                    },
                ],
                data: buffer,
                programId: new web3.PublicKey(REVIEW_PROGRAM_ID),
            });

            console.log("Instruction created:", {
                programId: instruction.programId.toBase58(),
                keys: instruction.keys.map(k => ({ pubkey: k.pubkey.toBase58(), isSigner: k.isSigner, isWritable: k.isWritable })),
                dataLength: instruction.data.length
            });

            const transaction = new web3.Transaction();
            transaction.add(instruction);
            console.log("Transaction created with", transaction.instructions.length, "instruction(s)");

            console.log("Sending transaction...");
            let txid = await sendTransaction(transaction, connection);
            console.log(`Transaction submitted: ${txid}`);
            console.log(`Explorer link: https://explorer.solana.com/tx/${txid}?cluster=devnet`);
            
            setTxid(
                `Transaction submitted: https://explorer.solana.com/tx/${txid}?cluster=devnet`
            );
            
            // Wait for confirmation
            console.log("Waiting for confirmation...");
            await connection.confirmTransaction(txid, 'confirmed');
            console.log('Transaction confirmed successfully!');
            
            // Refresh reviews after successful submission
            console.log("Fetching updated reviews...");
            const updatedReviews = await fetchReviews(REVIEW_PROGRAM_ID, connection);
            setReviews(updatedReviews);
            console.log("Reviews updated:", updatedReviews.length, "total reviews");
        } catch (e: any) {
            console.error('=== Transaction Error ===');
            console.error('Error type:', e.constructor.name);
            console.error('Error message:', e.message);
            console.error('Error stack:', e.stack);
            console.error('Full error object:', e);
            
            if (e.logs) {
                console.error('Transaction logs:', e.logs);
            }
            
            alert(`Error: ${e.message || JSON.stringify(e)}`);
        }
    };

    return (
        <main
            className={`flex min-h-screen flex-col items-center justify-between p-24 `}
        >
            <div className="z-10 max-w-5xl w-full items-center justify-between font-mono text-sm lg:flex">
                <AppBar />
            </div>

            <div className="after:absolute after:-z-20 after:h-[180px] after:w-[240px] after:translate-x-1/3 after:bg-gradient-conic after:from-sky-200 after:via-blue-200 after:blur-2xl after:content-[''] before:dark:bg-gradient-to-br before:dark:from-transparent before:dark:to-blue-700/10 after:dark:from-sky-900 after:dark:via-[#0141ff]/40 before:lg:h-[360px]">
                <ReviewForm
                    title={title}
                    description={description}
                    rating={rating}
                    setTitle={setTitle}
                    setDescription={setDescription}
                    setRating={setRating}
                    handleSubmit={handleSubmit}
                />
            </div>

            {txid && <div>{txid}</div>}

            <div className="mb-32 grid text-center lg:max-w-5xl lg:w-full lg:mb-0 lg:grid-cols-3 lg:text-left">
                {reviews &&
                    reviews.map((review) => {
                        return (
                            <ReviewCard key={review.title} review={review} />
                        );
                    })}
            </div>
        </main>
    );
}
